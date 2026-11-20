//! Local audio settings: quality and equalizer.

use super::widgets::*;
use super::*;
use std::sync::{Arc, Mutex};

const EQ_ROW_HEIGHT: f32 = 178.0;
const EQ_VALUE_HEIGHT: f32 = 16.0;
const EQ_SLIDER_HEIGHT: f32 = 144.0;
const EQ_TRACK_TOP: f32 = 8.0;
const EQ_TRACK_HEIGHT: f32 = 128.0;
const EQ_CURVE_TOP: f32 = EQ_VALUE_HEIGHT + EQ_TRACK_TOP;

fn equalizer_gain_at(y: gpui::Pixels, bounds: Option<&Bounds<gpui::Pixels>>) -> Option<f32> {
    let bounds = bounds?;
    let top = f32::from(bounds.origin.y) + EQ_TRACK_TOP;
    let value = 1.0 - ((f32::from(y) - top) / EQ_TRACK_HEIGHT).clamp(0.0, 1.0);
    Some(value * 24.0 - 12.0)
}

fn quality_label(quality: yandex_music::AudioQuality) -> &'static str {
    match quality {
        yandex_music::AudioQuality::Lossless => "Без потерь",
        yandex_music::AudioQuality::High => "Высокое",
        yandex_music::AudioQuality::Normal => "Обычное",
    }
}

fn quality_description(quality: yandex_music::AudioQuality) -> &'static str {
    match quality {
        yandex_music::AudioQuality::Lossless => "FLAC, если доступен; иначе MP3 320",
        yandex_music::AudioQuality::High => "MP3 320 кбит/с",
        yandex_music::AudioQuality::Normal => "MP3 до 192 кбит/с",
    }
}

impl RootView {
    pub(crate) fn settings_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.settings_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn settings_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let header = div()
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(
                div()
                    .text_size(px(text::DISPLAY))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Настройки"),
            )
            .child(
                div()
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child("Качество звука и эквалайзер сохраняются локально"),
            );

        let qualities = [
            yandex_music::AudioQuality::Lossless,
            yandex_music::AudioQuality::High,
            yandex_music::AudioQuality::Normal,
        ];
        let selected_quality = self.audio_quality;
        let quality_card = card(theme)
            .gap(px(space::SM))
            .child(section_title(theme, "Качество звука"))
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(
                        div()
                            .id("quality-select")
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(42.0))
                            .px(px(space::MD))
                            .rounded(px(9.0))
                            .bg(theme.colors.surface)
                            .border_1()
                            .border_color(if self.quality_select_open {
                                theme.colors.accent
                            } else {
                                theme.colors.border
                            })
                            .cursor_pointer()
                            .hover(|style| style.border_color(theme.colors.accent))
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.quality_select_open = !this.quality_select_open;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.0))
                                    .child(
                                        div()
                                            .text_size(px(text::SM))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(quality_label(selected_quality)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(text::XS))
                                            .text_color(theme.colors.text_muted)
                                            .child(quality_description(selected_quality)),
                                    ),
                            )
                            .child(
                                Icon::new(icons::CHEVRON_DOWN)
                                    .size(15.0)
                                    .color(theme.colors.text_muted),
                            ),
                    )
                    .when(self.quality_select_open, |element| {
                        let mut list = div()
                            .mt(px(4.0))
                            .w_full()
                            .rounded(px(9.0))
                            .bg(theme.colors.surface)
                            .border_1()
                            .border_color(theme.colors.border)
                            .overflow_hidden()
                            .p(px(4.0))
                            .flex()
                            .flex_col();
                        for quality in qualities {
                            let selected = quality == selected_quality;
                            list = list.child(
                                div()
                                    .id(ElementId::Name(SharedString::from(format!(
                                        "quality-option-{}",
                                        quality_label(quality)
                                    ))))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(space::MD))
                                    .py(px(space::SM))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(selected, |row| row.bg(theme.colors.accent_soft))
                                    .hover(|style| style.bg(theme.colors.surface_hover))
                                    .on_click(cx.listener(
                                        move |this, _: &gpui::ClickEvent, _, cx| {
                                            this.set_audio_quality(quality, cx);
                                            this.quality_select_open = false;
                                            cx.notify();
                                        },
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(1.0))
                                            .child(
                                                div()
                                                    .text_size(px(text::SM))
                                                    .child(quality_label(quality)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(text::XS))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(quality_description(quality)),
                                            ),
                                    )
                                    .when(selected, |row| {
                                        row.child(Badge::new("Выбрано").tone(BadgeTone::Accent))
                                    }),
                            );
                        }
                        element.child(list)
                    }),
            );

        let presets = [
            config::EqualizerPreset::Flat,
            config::EqualizerPreset::Bass,
            config::EqualizerPreset::Vocal,
            config::EqualizerPreset::Rock,
            config::EqualizerPreset::Electronic,
        ];
        let mut preset_row = div().flex().items_center().gap(px(6.0)).flex_wrap();
        for preset in presets {
            let selected = self.equalizer == preset;
            preset_row = preset_row.child(
                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "eq-preset-{}",
                        preset.label()
                    ))))
                    .px(px(space::SM))
                    .py(px(5.0))
                    .rounded_full()
                    .border_1()
                    .border_color(if selected {
                        theme.colors.accent
                    } else {
                        theme.colors.border
                    })
                    .when(selected, |chip| {
                        chip.bg(theme.colors.accent_soft)
                            .text_color(theme.colors.accent)
                    })
                    .when(!selected, |chip| {
                        chip.text_color(theme.colors.text_secondary)
                            .hover(|style| style.border_color(theme.colors.accent))
                    })
                    .cursor_pointer()
                    .text_size(px(text::XS))
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.set_equalizer(preset, cx);
                    }))
                    .child(preset.label()),
            );
        }

        let bands = ["60 Гц", "230 Гц", "910 Гц", "3.6 кГц", "14 кГц"];
        let gains = self.equalizer_gains;
        let curve_color = theme.colors.accent;
        let mut columns = div()
            .relative()
            .w_full()
            .h(px(EQ_ROW_HEIGHT))
            .flex()
            .items_start()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let width = f32::from(bounds.size.width);
                        let mut points = Vec::with_capacity(5);
                        for (index, gain) in gains.iter().enumerate() {
                            let x =
                                f32::from(bounds.origin.x) + width * ((index as f32 + 0.5) / 5.0);
                            let y = f32::from(bounds.origin.y)
                                + EQ_CURVE_TOP
                                + (1.0 - (gain + 12.0) / 24.0) * EQ_TRACK_HEIGHT;
                            points.push(point(px(x), px(y)));
                        }
                        let mut path = PathBuilder::stroke(px(2.0));
                        path.move_to(points[0]);
                        for pair in points.windows(2) {
                            let from = pair[0];
                            let to = pair[1];
                            let mid_x = (f32::from(from.x) + f32::from(to.x)) / 2.0;
                            path.cubic_bezier_to(
                                to,
                                point(px(mid_x), from.y),
                                point(px(mid_x), to.y),
                            );
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, curve_color);
                        }
                    },
                )
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full(),
            );

        for (index, band) in bands.iter().enumerate() {
            let gain = self.equalizer_gains[index];
            let value = ((gain + 12.0) / 24.0).clamp(0.0, 1.0);
            let slider_bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> =
                Arc::new(Mutex::new(None));
            let bounds_for_canvas = slider_bounds.clone();
            let bounds_for_down = slider_bounds.clone();
            let bounds_for_move = slider_bounds;
            let down_view = cx.entity();
            let move_view = cx.entity();
            let up_view = cx.entity();
            let up_out_view = cx.entity();

            let knob_top = EQ_TRACK_TOP + (1.0 - value) * EQ_TRACK_HEIGHT - 7.0;
            let slider = div()
                .relative()
                .w(px(48.0))
                .h(px(EQ_SLIDER_HEIGHT))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &gpui::MouseDownEvent, _, cx| {
                        let Ok(bounds) = bounds_for_down.lock() else {
                            return;
                        };
                        let Some(gain) = equalizer_gain_at(event.position.y, bounds.as_ref())
                        else {
                            return;
                        };
                        down_view.update(cx, |this, cx| {
                            this.begin_equalizer_drag(index, gain, cx);
                        });
                    },
                )
                .on_mouse_move(move |event: &gpui::MouseMoveEvent, _, cx| {
                    if !event.dragging() {
                        return;
                    }
                    let Ok(bounds) = bounds_for_move.lock() else {
                        return;
                    };
                    let Some(gain) = equalizer_gain_at(event.position.y, bounds.as_ref()) else {
                        return;
                    };
                    move_view.update(cx, |this, cx| {
                        this.drag_equalizer(index, gain, cx);
                    });
                })
                .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                    up_view.update(cx, |this, cx| {
                        this.end_equalizer_drag(index, cx);
                    });
                })
                .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                    up_out_view.update(cx, |this, cx| {
                        this.end_equalizer_drag(index, cx);
                    });
                })
                .child(
                    div()
                        .absolute()
                        .top(px(EQ_TRACK_TOP))
                        .left(px(22.0))
                        .w(px(4.0))
                        .h(px(EQ_TRACK_HEIGHT))
                        .rounded_full()
                        .bg(theme.colors.border_strong)
                        .overflow_hidden()
                        .child(
                            div()
                                .absolute()
                                .bottom(px(0.0))
                                .left(px(0.0))
                                .w_full()
                                .h(px(value * EQ_TRACK_HEIGHT))
                                .rounded_full()
                                .bg(theme.colors.accent),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(EQ_TRACK_TOP + EQ_TRACK_HEIGHT / 2.0))
                        .left(px(17.0))
                        .w(px(14.0))
                        .h(px(1.0))
                        .bg(theme.colors.border_strong),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(knob_top))
                        .left(px(17.0))
                        .size(px(14.0))
                        .rounded_full()
                        .bg(theme.colors.accent)
                        .border_2()
                        .border_color(theme.colors.surface_active),
                )
                .child(
                    canvas(
                        move |bounds, _, _| {
                            if let Ok(mut slot) = bounds_for_canvas.lock() {
                                *slot = Some(bounds);
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .size_full(),
                );

            columns = columns.child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .h(px(EQ_VALUE_HEIGHT))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("{gain:+.0}")),
                    )
                    .child(slider)
                    .child(
                        div()
                            .h(px(EQ_VALUE_HEIGHT))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .child(*band),
                    ),
            );
        }

        let eq_card = card(theme)
            .gap(px(space::SM))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_title(theme, "Эквалайзер"))
                    .when(
                        self.equalizer == config::EqualizerPreset::Custom,
                        |element| element.child(Badge::new("Свой").tone(BadgeTone::Outline)),
                    ),
            )
            .child(preset_row)
            .child(
                div()
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child("Пресеты задают значения; ползунки в дБ можно менять отдельно."),
            )
            .child(columns);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("settings-scroll")
            .overflow_y_scroll()
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(760.0))
                        .p(px(space::LG))
                        .flex()
                        .flex_col()
                        .gap(px(space::MD))
                        .child(header)
                        .child(quality_card)
                        .child(eq_card)
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Настройки хранятся в playback.json."),
                        ),
                ),
            )
            .into_any_element()
    }
}
