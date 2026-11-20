//! Local audio settings: quality and equalizer.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn settings_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.settings_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn settings_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let header = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(px(text::DISPLAY))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Настройки"),
            )
            .child(
                div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_secondary)
                    .child("Качество звука и эквалайзер сохраняются локально"),
            );

        let mut quality_card = card(theme)
            .child(section_title(theme, "Качество звука"))
            .child(
                div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_muted)
                    .child("Используется при загрузке новых треков."),
            );

        let qualities = [
            (
                yandex_music::AudioQuality::Lossless,
                "Без потерь",
                "FLAC, если доступен; иначе MP3 320 кбит/с",
            ),
            (
                yandex_music::AudioQuality::High,
                "Высокое",
                "MP3 320 кбит/с",
            ),
            (
                yandex_music::AudioQuality::Normal,
                "Обычное",
                "MP3 до 192 кбит/с — экономит трафик",
            ),
        ];
        for (quality, title, description) in qualities {
            let selected = self.audio_quality == quality;
            let mut row = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "quality-{}",
                    match quality {
                        yandex_music::AudioQuality::Lossless => "lossless",
                        yandex_music::AudioQuality::High => "high",
                        yandex_music::AudioQuality::Normal => "normal",
                    }
                ))))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(space::LG))
                .p(px(space::MD))
                .rounded(px(10.0))
                .border_1()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.set_audio_quality(quality, cx);
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(text::BASE))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child(description),
                        ),
                );
            if selected {
                row = row
                    .bg(theme.colors.accent_soft)
                    .border_color(theme.colors.accent)
                    .child(Badge::new("Выбрано").tone(BadgeTone::Accent));
            } else {
                row = row
                    .bg(theme.colors.surface)
                    .border_color(theme.colors.border)
                    .hover(|style| style.border_color(theme.colors.accent));
            }
            quality_card = quality_card.child(row);
        }

        let mut eq_card = card(theme).child(section_title(theme, "Эквалайзер")).child(
            div()
                .text_size(px(text::SM))
                .text_color(theme.colors.text_muted)
                .child("Применяется сразу к текущему треку."),
        );

        let presets = [
            config::EqualizerPreset::Flat,
            config::EqualizerPreset::Bass,
            config::EqualizerPreset::Vocal,
            config::EqualizerPreset::Rock,
            config::EqualizerPreset::Electronic,
        ];
        let mut grid = div().w_full().grid().grid_cols(2).gap(px(space::SM));
        for preset in presets {
            let selected = self.equalizer == preset;
            let mut cell = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "eq-{}",
                    preset.label()
                ))))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(space::MD))
                .rounded(px(10.0))
                .border_1()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.set_equalizer(preset, cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(text::BASE))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(preset.label()),
                        )
                        .when(selected, |element| {
                            element.child(Badge::new("Активен").tone(BadgeTone::Accent))
                        }),
                )
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .child(preset.description()),
                );
            if selected {
                cell = cell
                    .bg(theme.colors.accent_soft)
                    .border_color(theme.colors.accent);
            } else {
                cell = cell
                    .bg(theme.colors.surface)
                    .border_color(theme.colors.border)
                    .hover(|style| style.border_color(theme.colors.accent));
            }
            grid = grid.child(cell);
        }
        eq_card = eq_card.child(grid);

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
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(quality_card)
                        .child(eq_card)
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child(
                                    "Настройки хранятся в playback.json в каталоге конфигурации.",
                                ),
                        ),
                ),
            )
            .into_any_element()
    }
}
