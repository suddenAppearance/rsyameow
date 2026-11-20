//! Liked tracks page rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn liked_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.liked_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn liked_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let total = self
            .profile
            .as_ref()
            .map(|profile| profile.total_liked)
            .unwrap_or(0);

        let header = div()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(space::LG))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text::DISPLAY))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Любимые треки"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Вся коллекция «Мне нравится»"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        IconButton::new("refresh-liked", icons::REFRESH)
                            .size(38.0)
                            .icon_size(16.0)
                            .variant(ButtonVariant::Secondary)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                if this.demo {
                                    this.start_demo(cx);
                                } else {
                                    this.retry_profile(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("shuffle-liked", "Перемешать")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::SHUFFLE)
                            .loading(self.player.loading)
                            .disabled(total == 0)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.play_random(cx);
                            })),
                    ),
            );

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("liked-scroll")
            .overflow_y_scroll()
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(self.liked_section(theme, cx)),
                ),
            )
            .into_any_element()
    }

    pub(crate) fn liked_summary(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let total = self
            .profile
            .as_ref()
            .map(|profile| profile.total_liked)
            .unwrap_or(0);

        card(theme)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(space::LG))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(
                                div()
                                    .size(px(42.0))
                                    .rounded(px(12.0))
                                    .bg(theme.colors.accent_soft)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(icons::HEART)
                                            .size(20.0)
                                            .color(theme.colors.accent),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.0))
                                    .child(
                                        div()
                                            .text_size(px(text::BASE))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Любимые треки"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(text::SM))
                                            .text_color(theme.colors.text_muted)
                                            .child(format::plural(
                                                total as u64,
                                                "трек в коллекции",
                                                "трека в коллекции",
                                                "треков в коллекции",
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        Button::new("open-liked", "Открыть")
                            .variant(ButtonVariant::Secondary)
                            .trailing_icon(icons::CHEVRON_RIGHT)
                            .on_click(
                                cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.open_liked(cx)
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn liked_section(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(profile) = &self.profile else {
            return div().into_any_element();
        };

        let mut section = card(theme).child(
            div().flex().items_center().justify_between().child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::MD))
                    .child(section_title(theme, "Треки"))
                    .child(
                        Badge::new(format::plural(
                            profile.total_liked as u64,
                            "трек",
                            "трека",
                            "треков",
                        ))
                        .tone(BadgeTone::Neutral),
                    ),
            ),
        );

        if profile.sample.is_empty() {
            section = section.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::MD))
                    .py(px(space::XXL))
                    .child(
                        div()
                            .size(px(52.0))
                            .rounded(px(16.0))
                            .bg(theme.colors.surface_active)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(icons::HEART)
                                    .size(24.0)
                                    .color(theme.colors.text_muted),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(text::LG))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Здесь пока пусто"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .text_center()
                            .max_w(px(360.0))
                            .child(
                                "Поставьте «Мне нравится» нескольким трекам в Яндекс Музыке и обновите профиль.",
                            ),
                    )
                    .child(
                        Button::new("refresh-empty", "Обновить профиль")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::REFRESH)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.retry_profile(cx);
                            })),
                    ),
            );
        } else {
            let current_id = self.player.track.as_ref().map(|track| track.id.clone());
            let playing = self.player.playing;

            for (index, track) in profile.sample.iter().enumerate() {
                let is_current = current_id.as_deref() == Some(track.id.as_str());
                let cover = self.covers.get(&track.id).cloned();
                let row = TrackRow::new(
                    ElementId::Name(SharedString::from(format!("liked-{}-{index}", track.id))),
                    index + 1,
                    track.display_title().to_string(),
                    track.artist_names(),
                    format::duration_or_dash(track.duration()),
                )
                .cover(cover)
                .current(is_current, playing)
                .animation_phase(self.animation_phase)
                .explicit(track.explicit.unwrap_or(false))
                .on_play(cx.listener({
                    let track = track.clone();
                    move |this, _: &gpui::ClickEvent, _, cx| {
                        this.play_track(track.clone(), cx);
                    }
                }));

                let row = track_navigation(row, track, cx);
                section = section.child(row);
            }

            if profile.total_liked > profile.sample.len() {
                section = section.child(
                    div()
                        .pt(px(space::SM))
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child(format!(
                            "Показано {} из {} треков",
                            profile.sample.len(),
                            profile.total_liked
                        )),
                );
            }
        }

        section.into_any_element()
    }
}
