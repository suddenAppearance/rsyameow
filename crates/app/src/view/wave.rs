//! Full-page personal wave poster.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn wave_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.wave_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn wave_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let active = self.queue_source == QueueSource::Wave;
        let track = if active {
            self.player.track.clone()
        } else {
            None
        };
        let palette = track
            .as_ref()
            .map(|track| self.track_palette(&track.id))
            .unwrap_or_else(|| self.track_palette(""));
        let backdrop = track
            .as_ref()
            .and_then(|track| self.track_backdrop(&track.id));
        let loading = self.wave.loading;
        let error = self.wave.error.clone();

        let content = if let Some(track) = track {
            let cover = self.player.cover.clone();
            let cover_size = if self.viewport_height < 760.0 {
                220.0
            } else {
                300.0
            };
            let cover_element = match cover {
                Some(image) => img(ImageSource::Image(image))
                    .size(px(cover_size))
                    .rounded(px(24.0))
                    .object_fit(ObjectFit::Cover)
                    .shadow_lg()
                    .into_any_element(),
                None => div()
                    .size(px(cover_size))
                    .rounded(px(24.0))
                    .bg(theme.colors.surface_active)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(icons::MUSIC)
                            .size(56.0)
                            .color(theme.colors.text_muted),
                    )
                    .into_any_element(),
            };
            let playing = self.player.playing;
            let progress = self.player.progress();
            let elapsed = format::duration(self.player.position);
            let duration = format::duration(track.duration());
            let liked = self.is_liked(&track);

            let mut poster =
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::MD))
                    .child(Badge::new("МОЯ ВОЛНА").tone(BadgeTone::Accent))
                    .child(cover_element)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .pt(px(space::SM))
                            .max_w(px(720.0))
                            .child(
                                div()
                                    .flex_shrink()
                                    .min_w(px(0.0))
                                    .text_size(px(if self.viewport_height < 760.0 {
                                        28.0
                                    } else {
                                        38.0
                                    }))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_center()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(track.display_title().to_string()),
                            )
                            .when(track.explicit.unwrap_or(false), |element| {
                                element.child(
                                    Icon::new(icons::EXPLICIT)
                                        .size(18.0)
                                        .color(rgba(0xFFFFFFB3)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(text::LG))
                            .text_color(theme.colors.text_secondary)
                            .text_center()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .max_w(px(620.0))
                            .child(track.artist_names()),
                    )
                    .child(
                        div()
                            .w(px(420.0))
                            .max_w(relative(0.82))
                            .pt(px(space::MD))
                            .flex()
                            .flex_col()
                            .gap(px(space::XS))
                            .child({
                                let view = cx.entity();
                                Slider::new(progress).width(420.0).on_change(
                                    move |fraction, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.seek_to(fraction, cx);
                                        });
                                    },
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(elapsed)
                                    .child(duration),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .pt(px(space::SM))
                            .child(div().w(px(44.0)).flex_shrink_0())
                            .child(
                                IconButton::new("wave-previous", icons::PREVIOUS)
                                    .size(44.0)
                                    .icon_size(18.0)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.play_previous(cx)
                                    })),
                            )
                            .child(
                                IconButton::new(
                                    "wave-toggle",
                                    if playing { icons::PAUSE } else { icons::PLAY },
                                )
                                .size(62.0)
                                .icon_size(24.0)
                                .variant(ButtonVariant::Primary)
                                .disabled(self.player.loading)
                                .on_click(cx.listener(
                                    |this, _: &gpui::ClickEvent, _, cx| this.toggle_playback(cx),
                                )),
                            )
                            .child(
                                IconButton::new("wave-next", icons::NEXT)
                                    .size(44.0)
                                    .icon_size(18.0)
                                    .variant(ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.play_next(cx)
                                    })),
                            )
                            .child(
                                IconButton::new(
                                    "wave-like",
                                    if liked {
                                        icons::HEART
                                    } else {
                                        icons::HEART_OUTLINE
                                    },
                                )
                                .size(44.0)
                                .icon_size(18.0)
                                .variant(if liked {
                                    ButtonVariant::Soft
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .on_click(cx.listener(
                                    |this, _: &gpui::ClickEvent, _, cx| this.toggle_like(cx),
                                )),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(space::SM))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .child("Волна продолжится автоматически"),
                    );

            if let Some(error) = &error {
                poster = poster.child(error_banner(theme, error));
            }
            poster
        } else {
            let mut intro = div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(space::LG))
                .child(
                    div()
                        .size(px(112.0))
                        .rounded(px(30.0))
                        .bg(theme.colors.accent_soft)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(icons::SPARKLES)
                                .size(52.0)
                                .color(theme.colors.accent),
                        ),
                )
                .child(
                    div()
                        .text_size(px(40.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Моя волна"),
                )
                .child(
                    Button::new("wave-start", "Включить волну")
                        .variant(ButtonVariant::Primary)
                        .icon(icons::SPARKLES)
                        .loading(loading)
                        .on_click(
                            cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.play_wave(cx)),
                        ),
                );

            if loading {
                intro = intro.child(Spinner::new().size(18.0).color(theme.colors.accent));
            }
            if let Some(error) = &error {
                intro = intro.child(error_banner(theme, error));
            }
            intro
        };

        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_hidden()
            .bg(palette.0)
            .child(immersive_background(palette, backdrop))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.0))
                    .items_center()
                    .justify_center()
                    .p(px(space::XL))
                    .child(content),
            )
            .into_any_element()
    }
}
