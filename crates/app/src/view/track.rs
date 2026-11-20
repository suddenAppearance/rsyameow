//! Selected/current track page, lyrics and immersive background.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn track_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.track_content(theme, false, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn now_playing_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.track_content(theme, true, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn track_content(
        &mut self,
        theme: &Theme,
        current_only: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let track = if current_only {
            self.player.track.clone()
        } else {
            self.page_track
                .clone()
                .or_else(|| self.player.track.clone())
        };
        let Some(track) = track else {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(space::LG))
                .child(
                    Icon::new(icons::MUSIC)
                        .size(36.0)
                        .color(theme.colors.text_muted),
                )
                .child(
                    div()
                        .text_size(px(text::LG))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Сейчас ничего не играет"),
                )
                .child(
                    Button::new("track-open-liked", "Открыть любимые треки")
                        .variant(ButtonVariant::Primary)
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                            this.open_liked(cx);
                        })),
                )
                .into_any_element();
        };

        let palette = self.track_palette(&track.id);
        if current_only {
            self.load_track_lyrics(track.clone(), cx);
            self.load_track_backdrop(track.clone(), cx);
        }
        let backdrop = self.track_backdrop(&track.id);
        let is_current = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);
        let playing = is_current && self.player.playing;
        let loading = is_current && self.player.loading;
        let liked = self.is_liked(&track);
        let quality = if is_current {
            self.player.quality.clone()
        } else {
            None
        };
        let position = format::duration_or_dash(self.player.position);
        let duration = format::duration_or_dash(track.duration());
        let year = track.albums.first().and_then(|album| album.year);
        let album = track.albums.first().cloned();
        let artists = track.artists.clone();
        let cover = if is_current {
            self.player.cover.clone()
        } else {
            self.covers.get(&track.id).cloned()
        };

        let lyrics = self.track_lyrics.get(&track.id).cloned().flatten();
        let explicit = track.explicit.unwrap_or(false);

        let mut artist_values = div().flex().items_center().gap(px(space::XS)).flex_wrap();
        let mut artist_links = 0usize;
        for (index, artist) in artists.iter().enumerate() {
            let Some(id) = artist.id else {
                continue;
            };
            artist_links += 1;
            let artist_for_click = artist.clone();
            artist_values = artist_values.child(
                Button::new(
                    ElementId::Name(SharedString::from(format!("track-artist-{id}-{index}"))),
                    artist.display_name().to_string(),
                )
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.open_artist(id, Some(artist_for_click.clone()), cx);
                })),
            );
        }
        if artist_links == 0 && !artists.is_empty() {
            artist_values = artist_values.child(
                div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.white)
                    .child(track.artist_names()),
            );
        }

        let album_values = album.as_ref().map(|album| {
            let album_button = album.id.map(|id| {
                let album = album.clone();
                Button::new(
                    ElementId::Name(SharedString::from(format!("track-album-{id}"))),
                    album.display_title().to_string(),
                )
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.open_album(id, Some(album.clone()), cx);
                }))
            });
            let values = match album_button {
                Some(button) => div().child(button).into_any_element(),
                None => div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.white)
                    .child(album.display_title().to_string())
                    .into_any_element(),
            };
            div()
                .flex()
                .items_center()
                .gap(px(space::SM))
                .flex_wrap()
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(rgba(0xFFFFFF80))
                        .child("Альбом"),
                )
                .child(values)
                .when_some(year, |element, year| {
                    element.child(
                        div()
                            .text_size(px(text::XS))
                            .text_color(rgba(0xFFFFFF66))
                            .child(year.to_string()),
                    )
                })
        });

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-from-track", "Назад")
                    .variant(ButtonVariant::Ghost)
                    .icon(icons::CHEVRON_LEFT)
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.navigate_back(cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        Button::new(
                            "track-toggle",
                            if playing {
                                "Пауза"
                            } else {
                                "Слушать"
                            },
                        )
                        .variant(ButtonVariant::Primary)
                        .icon(if playing { icons::PAUSE } else { icons::PLAY })
                        .loading(loading)
                        .on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                if current_only {
                                    this.toggle_playback(cx);
                                } else {
                                    this.play_page_track(cx);
                                }
                            },
                        )),
                    )
                    .child(
                        IconButton::new(
                            "track-like",
                            if liked {
                                icons::HEART
                            } else {
                                icons::HEART_OUTLINE
                            },
                        )
                        .size(38.0)
                        .icon_size(17.0)
                        .variant(if liked {
                            ButtonVariant::Soft
                        } else {
                            ButtonVariant::Secondary
                        })
                        .on_click(cx.listener({
                            let track = track.clone();
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                this.toggle_like_track(track.clone(), cx);
                            }
                        })),
                    )
                    .child(
                        IconButton::new("track-queue", icons::QUEUE)
                            .size(38.0)
                            .icon_size(17.0)
                            .variant(if self.queue_open {
                                ButtonVariant::Soft
                            } else {
                                ButtonVariant::Secondary
                            })
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.toggle_queue(cx);
                            })),
                    ),
            );

        let hero = glass_card(theme)
            .flex_row()
            .items_center()
            .gap(px(space::LG))
            .p(px(space::LG))
            .child(search_thumb(theme, cover, icons::MUSIC, 160.0, 18.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(space::XS))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .min_w(px(0.0))
                            .child(
                                div()
                                    .flex_shrink()
                                    .min_w(px(0.0))
                                    .text_size(px(30.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(px(34.0))
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(track.display_title().to_string()),
                            )
                            .when(explicit, |element| {
                                element.child(
                                    Icon::new(icons::EXPLICIT)
                                        .size(17.0)
                                        .color(rgba(0xFFFFFFB3)),
                                )
                            }),
                    )
                    .when(!artists.is_empty(), |element| {
                        element.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(space::SM))
                                .flex_wrap()
                                .child(
                                    div()
                                        .text_size(px(text::XS))
                                        .text_color(rgba(0xFFFFFF80))
                                        .child("Исполнитель"),
                                )
                                .child(artist_values),
                        )
                    })
                    .when_some(album_values, |element, album| element.child(album))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::SM))
                            .pt(px(space::XS))
                            .text_size(px(text::SM))
                            .text_color(rgba(0xFFFFFFB3))
                            .child(duration.clone())
                            .when(is_current, |element| {
                                element
                                    .child(div().text_color(rgba(0xFFFFFF4D)).child("·"))
                                    .child(format!("прослушано {position}"))
                            })
                            .when_some(quality.clone(), |element, quality| {
                                element.child(Badge::new(quality).tone(BadgeTone::Outline))
                            }),
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));
        if is_current && let Some(error) = &self.player.error {
            body = body.child(error_banner(theme, error));
        }
        if let Some(lyrics) = lyrics {
            let is_long = lyrics.lines().count() > 12 || lyrics.chars().count() > 700;
            let collapsed = is_long && !self.lyrics_expanded;
            let text_color = if collapsed {
                rgba(0xFFFFFFA6)
            } else {
                rgba(0xFFFFFFD9)
            };
            let text = div()
                .pt(px(space::SM))
                .text_size(px(15.0))
                .line_height(px(24.0))
                .text_color(text_color)
                .whitespace_normal()
                .child(lyrics);

            let mut card = glass_card(theme).child(section_title(theme, "Текст трека"));
            if collapsed {
                card = card.child(
                    div()
                        .relative()
                        .max_h(px(288.0))
                        .overflow_hidden()
                        .child(text)
                        .child(
                            div()
                                .absolute()
                                .bottom(px(0.0))
                                .left(px(0.0))
                                .right(px(0.0))
                                .h(px(96.0))
                                .bg(linear_gradient(
                                    0.0,
                                    linear_color_stop(rgba(0x0A0A0A00), 0.0),
                                    linear_color_stop(rgba(0x0A0A0AF5), 1.0),
                                )),
                        ),
                );
            } else {
                card = card.child(text);
            }

            if is_long {
                card = card.child(
                    div().flex().justify_center().pt(px(space::SM)).child(
                        Button::new(
                            "lyrics-toggle",
                            if collapsed {
                                "Показать полностью"
                            } else {
                                "Свернуть"
                            },
                        )
                        .variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(
                            |this, _: &gpui::ClickEvent, _, cx| {
                                this.toggle_lyrics(cx);
                            },
                        )),
                    ),
                );
            }
            body = body.child(card);
        }

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
                    .id("track-scroll")
                    .overflow_y_scroll()
                    .child(
                        div().w_full().flex().justify_center().child(
                            div()
                                .w_full()
                                .max_w(px(1000.0))
                                .p(px(space::XL))
                                .flex()
                                .flex_col()
                                .gap(px(space::LG))
                                .child(header)
                                .child(hero)
                                .child(body),
                        ),
                    ),
            )
            .into_any_element()
    }
}
