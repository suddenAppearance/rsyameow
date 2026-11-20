//! Local audio settings: quality and equalizer.

use super::widgets::*;
use super::*;

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
        let mut sliders = div().flex().flex_col().gap(px(2.0));
        for (index, band) in bands.iter().enumerate() {
            let gain = self.equalizer_gains[index];
            let value = ((gain + 12.0) / 24.0).clamp(0.0, 1.0);
            let preview_view = cx.entity();
            let commit_view = cx.entity();
            sliders = sliders.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .h(px(28.0))
                    .child(
                        div()
                            .w(px(58.0))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .child(*band),
                    )
                    .child(
                        Slider::new(value)
                            .width(340.0)
                            .on_change(move |value, _, cx| {
                                preview_view.update(cx, |this, cx| {
                                    this.set_equalizer_band(index, value * 24.0 - 12.0, cx);
                                });
                            })
                            .on_commit(move |_, _, cx| {
                                commit_view.update(cx, |this, cx| {
                                    this.commit_equalizer(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .w(px(52.0))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_secondary)
                            .text_right()
                            .child(format!("{gain:+.0} дБ")),
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
                    .child("Пресеты задают значения; ползунки можно менять отдельно."),
            )
            .child(sliders);

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
