//! Artist page and full discography rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn artist_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.artist_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn artist_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let artist = self.artist.artist.clone();
        let tracks = self.artist.tracks.clone();
        let albums = self.artist.albums.clone();
        let loading = self.artist.loading;
        let error = self.artist.error.clone();
        let artist_id = self.artist.id;
        let background_key = artist_id
            .map(|id| format!("artist-{id}"))
            .unwrap_or_default();
        let palette = self.track_palette(&background_key);
        let backdrop = self.track_backdrop(&background_key);
        let cover =
            artist_id.and_then(|id| self.search_images.get(&format!("artist-{id}")).cloned());
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let name = artist
            .as_ref()
            .map(|artist| artist.display_name().to_string())
            .unwrap_or_else(|| "Артист".to_string());
        let genres = artist
            .as_ref()
            .map(|artist| artist.genres.clone())
            .unwrap_or_default();
        let likes = artist
            .as_ref()
            .and_then(|artist| artist.likes_count)
            .unwrap_or(0);
        let track_count = artist
            .as_ref()
            .and_then(|artist| artist.counts.as_ref().map(|counts| counts.tracks))
            .filter(|count| *count > 0)
            .unwrap_or(tracks.len() as u64);
        let album_count = artist
            .as_ref()
            .and_then(|artist| artist.counts.as_ref().map(|counts| counts.direct_albums))
            .filter(|count| *count > 0)
            .unwrap_or(albums.len() as u64);

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-from-artist", "Назад")
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
                        Button::new("artist-play", "Слушать")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::PLAY)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener({
                                let tracks = tracks.clone();
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_context_all(tracks.clone(), false, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("artist-shuffle", "Перемешать")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::SHUFFLE)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener({
                                let tracks = tracks.clone();
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_context_all(tracks.clone(), true, cx);
                                }
                            })),
                    ),
            );

        let hero = glass_card(theme)
            .flex_row()
            .items_center()
            .gap(px(space::XL))
            .p(px(space::XL))
            .child(search_thumb(theme, cover, icons::USER, 168.0, 18.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .text_size(px(34.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .line_height(px(38.0))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(name),
                    )
                    .when(!genres.is_empty(), |element| {
                        element.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(space::XS))
                                .flex_wrap()
                                .children(
                                    genres
                                        .iter()
                                        .take(4)
                                        .map(|genre| {
                                            Badge::new(genre.clone()).tone(BadgeTone::Outline)
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::XL))
                            .pt(px(space::SM))
                            .child(Stat::new(
                                format::plural(track_count, "трек", "трека", "треков"),
                                "в каталоге",
                            ))
                            .child(divider_vertical(theme))
                            .child(Stat::new(
                                format::plural(album_count, "альбом", "альбома", "альбомов"),
                                "дискография",
                            ))
                            .when(likes > 0, |element| {
                                element
                                    .child(divider_vertical(theme))
                                    .child(Stat::new(format::count(likes), "лайков"))
                            }),
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if loading {
            body = body.child(
                glass_card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем артиста…"),
                    ),
            );
        } else {
            if let Some(error) = error {
                body = body.child(error_banner(theme, &error));
            }

            let mut tracks_card = glass_card(theme).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(section_title(theme, "Треки"))
                    .child(
                        Badge::new(format::plural(track_count, "трек", "трека", "треков"))
                            .tone(BadgeTone::Neutral),
                    ),
            );

            if tracks.is_empty() {
                tracks_card = tracks_card.child(
                    div()
                        .py(px(space::XL))
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child("У этого артиста пока нет доступных треков."),
                );
            } else {
                let context = tracks.clone();
                for (index, track) in tracks.iter().take(10).enumerate() {
                    let is_current = current_id.as_deref() == Some(track.id.as_str());
                    let cover = self.covers.get(&track.id).cloned();
                    let row = TrackRow::new(
                        ElementId::Name(SharedString::from(format!(
                            "artist-track-{}-{index}",
                            track.id
                        ))),
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
                        let context = context.clone();
                        move |this, _: &gpui::ClickEvent, _, cx| {
                            this.play_context_track(track.clone(), context.clone(), cx);
                        }
                    }));

                    let row = track_navigation(row, track, cx);
                    tracks_card = tracks_card.child(row);
                }
                let preview_count = tracks.len().min(10);
                if track_count as usize > preview_count {
                    tracks_card = tracks_card.child(
                        div()
                            .pt(px(space::SM))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .text_center()
                            .child(format!("Показано {preview_count} из {track_count} треков")),
                    );
                }
            }

            tracks_card = tracks_card.child(
                div().pt(px(space::SM)).flex().justify_center().child(
                    Button::new("artist-all-tracks", "Все треки")
                        .variant(ButtonVariant::Ghost)
                        .trailing_icon(icons::CHEVRON_RIGHT)
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                            this.open_artist_tracks(cx);
                        })),
                ),
            );

            body = body.child(tracks_card);

            if !albums.is_empty() {
                let mut strip = div().flex().gap(px(space::LG));
                for album in albums.iter().take(10) {
                    let id = album.id.unwrap_or(0);
                    let cover = self.search_images.get(&format!("album-{id}")).cloned();
                    let title = album.display_title().to_string();
                    let meta = match (album.year, album.track_count) {
                        (Some(year), Some(count)) => format!(
                            "{year} · {}",
                            format::plural(count, "трек", "трека", "треков")
                        ),
                        (Some(year), None) => year.to_string(),
                        (None, Some(count)) => format::plural(count, "трек", "трека", "треков"),
                        (None, None) => "Альбом".to_string(),
                    };
                    let album_for_click = album.clone();

                    strip = strip.child(
                        div()
                            .id(ElementId::Name(SharedString::from(format!(
                                "artist-album-{id}"
                            ))))
                            .w(px(180.0))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .gap(px(space::SM))
                            .p(px(space::SM))
                            .rounded(px(14.0))
                            .bg(theme.colors.surface)
                            .border_1()
                            .border_color(theme.colors.border)
                            .cursor_pointer()
                            .hover(|style| {
                                style
                                    .bg(theme.colors.surface_hover)
                                    .border_color(theme.colors.accent)
                            })
                            .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                                this.open_album(id, Some(album_for_click.clone()), cx);
                            }))
                            .child(playlist_cover(theme, cover, 130.0))
                            .child(
                                div()
                                    .px(px(space::XS))
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.0))
                                    .child(
                                        div()
                                            .text_size(px(text::BASE))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(text::XS))
                                            .text_color(theme.colors.text_muted)
                                            .child(meta),
                                    ),
                            ),
                    );
                }

                body = body
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(section_title(theme, "Альбомы"))
                            .child(
                                Badge::new(format::plural(
                                    album_count,
                                    "альбом",
                                    "альбома",
                                    "альбомов",
                                ))
                                .tone(BadgeTone::Neutral),
                            ),
                    )
                    .child(
                        div()
                            .id("artist-albums-strip")
                            .w_full()
                            .overflow_x_scroll()
                            .child(strip),
                    )
                    .child(
                        div().flex().justify_center().child(
                            Button::new("artist-all-albums", "Все альбомы")
                                .variant(ButtonVariant::Ghost)
                                .trailing_icon(icons::CHEVRON_RIGHT)
                                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.open_artist_albums(cx);
                                })),
                        ),
                    );
            }
        }

        immersive_scroll_page(
            palette,
            backdrop,
            "artist-scroll",
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(hero)
                        .child(body),
                )
                .into_any_element(),
        )
    }

    pub(crate) fn artist_tracks_view(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = self.artist_tracks_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn artist_tracks_content(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let background_key = self
            .artist
            .id
            .map(|id| format!("artist-{id}"))
            .unwrap_or_default();
        let palette = self.track_palette(&background_key);
        let backdrop = self.track_backdrop(&background_key);
        let artist_name = self
            .artist
            .artist
            .as_ref()
            .map(|artist| artist.display_name().to_string())
            .unwrap_or_else(|| "Артист".to_string());
        let tracks = self.artist.all_tracks.clone();
        let loading = self.artist.all_tracks_loading;
        let error = self.artist.all_tracks_error.clone();
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;
        let track_count = self
            .artist
            .artist
            .as_ref()
            .and_then(|artist| artist.counts.as_ref().map(|counts| counts.tracks))
            .filter(|count| *count > 0)
            .unwrap_or(tracks.len() as u64);

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-from-artist-tracks", "Назад")
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
                        Button::new("artist-tracks-play", "Слушать")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::PLAY)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener({
                                let tracks = tracks.clone();
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_context_all(tracks.clone(), false, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("artist-tracks-shuffle", "Перемешать")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::SHUFFLE)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener({
                                let tracks = tracks.clone();
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_context_all(tracks.clone(), true, cx);
                                }
                            })),
                    ),
            );

        let mut list = glass_card(theme).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(section_title(theme, "Все треки"))
                .child(
                    Badge::new(format::plural(track_count, "трек", "трека", "треков"))
                        .tone(BadgeTone::Neutral),
                ),
        );
        if let Some(error) = error {
            list = list.child(error_banner(theme, &error));
        }
        if loading && tracks.is_empty() {
            list = list.child(
                div()
                    .py(px(space::XXL))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::SM))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем треки артиста…"),
                    ),
            );
        } else if tracks.is_empty() {
            list = list.child(
                div()
                    .py(px(space::XL))
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_muted)
                    .text_center()
                    .child("У этого артиста пока нет доступных треков."),
            );
        } else {
            let context = tracks.clone();
            for (index, track) in tracks.iter().enumerate() {
                let is_current = current_id.as_deref() == Some(track.id.as_str());
                let cover = self.covers.get(&track.id).cloned();
                let mut row = TrackRow::new(
                    ElementId::Name(SharedString::from(format!(
                        "artist-all-track-{}-{index}",
                        track.id
                    ))),
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
                    let context = context.clone();
                    move |this, _: &gpui::ClickEvent, _, cx| {
                        this.play_context_track(track.clone(), context.clone(), cx);
                    }
                }));
                row = track_navigation(row, track, cx);
                list = list.child(row);
            }
        }

        immersive_scroll_page(
            palette,
            backdrop,
            "artist-tracks-scroll",
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(text::DISPLAY))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .child("Треки артиста"),
                                )
                                .child(
                                    div()
                                        .text_size(px(text::SM))
                                        .text_color(theme.colors.text_secondary)
                                        .child(artist_name),
                                ),
                        )
                        .child(list),
                )
                .into_any_element(),
        )
    }

    pub(crate) fn artist_albums_view(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = self.artist_albums_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn artist_albums_content(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let background_key = self
            .artist
            .id
            .map(|id| format!("artist-{id}"))
            .unwrap_or_default();
        let palette = self.track_palette(&background_key);
        let backdrop = self.track_backdrop(&background_key);
        let artist_name = self
            .artist
            .artist
            .as_ref()
            .map(|artist| artist.display_name().to_string())
            .unwrap_or_else(|| "Артист".to_string());
        let albums = self.artist.all_albums.clone();
        let loading = self.artist.all_albums_loading;
        let error = self.artist.all_albums_error.clone();

        let header = div().flex().items_center().child(
            Button::new("back-from-artist-albums", "Назад")
                .variant(ButtonVariant::Ghost)
                .icon(icons::CHEVRON_LEFT)
                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                    this.navigate_back(cx);
                })),
        );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));
        if let Some(error) = error {
            body = body.child(error_banner(theme, &error));
        }
        if loading && albums.is_empty() {
            body = body.child(
                glass_card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем альбомы…"),
                    ),
            );
        } else if albums.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::MUSIC,
                "Альбомов пока нет",
                "У этого артиста нет доступных альбомов.",
            ));
        } else {
            let columns = if self.viewport_width < 1100.0 { 2 } else { 3 };
            let mut grid = div().w_full().grid().grid_cols(columns).gap(px(space::LG));
            for album in &albums {
                let id = album.id.unwrap_or(0);
                let cover = self.search_images.get(&format!("album-{id}")).cloned();
                let title = album.display_title().to_string();
                let meta = match (album.year, album.track_count) {
                    (Some(year), Some(count)) => format!(
                        "{year} · {}",
                        format::plural(count, "трек", "трека", "треков")
                    ),
                    (Some(year), None) => year.to_string(),
                    (None, Some(count)) => format::plural(count, "трек", "трека", "треков"),
                    (None, None) => "Альбом".to_string(),
                };
                let album_for_click = album.clone();
                grid = grid.child(
                    div()
                        .id(ElementId::Name(SharedString::from(format!(
                            "artist-all-album-{id}"
                        ))))
                        .flex()
                        .flex_col()
                        .gap(px(space::SM))
                        .p(px(space::SM))
                        .rounded(px(14.0))
                        .bg(theme.colors.surface)
                        .border_1()
                        .border_color(theme.colors.border)
                        .cursor_pointer()
                        .hover(|style| {
                            style
                                .bg(theme.colors.surface_hover)
                                .border_color(theme.colors.accent)
                        })
                        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                            this.open_album(id, Some(album_for_click.clone()), cx);
                        }))
                        .child(playlist_cover(theme, cover, 150.0))
                        .child(
                            div()
                                .px(px(space::XS))
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .text_size(px(text::BASE))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(text::XS))
                                        .text_color(theme.colors.text_muted)
                                        .child(meta),
                                ),
                        ),
                );
            }
            body = body.child(grid);
        }

        immersive_scroll_page(
            palette,
            backdrop,
            "artist-albums-scroll",
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(text::DISPLAY))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .child("Альбомы артиста"),
                                )
                                .child(
                                    div()
                                        .text_size(px(text::SM))
                                        .text_color(theme.colors.text_secondary)
                                        .child(artist_name),
                                ),
                        )
                        .child(body),
                )
                .into_any_element(),
        )
    }
}
