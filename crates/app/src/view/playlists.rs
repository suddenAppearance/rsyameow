//! Playlists and playlist-detail rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn playlists_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.playlists_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    pub(crate) fn playlist_detail_view(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let main = self.playlist_detail_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    pub(crate) fn playlists_content(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let items = self.playlists.items.clone();
        let loading = self.playlists.loading;
        let loaded = self.playlists.loaded;
        let error = self.playlists.error.clone();
        let palette = items
            .first()
            .map(|playlist| self.playlist_palette(playlist))
            .unwrap_or_else(default_palette);
        let backdrop = items
            .first()
            .and_then(|playlist| self.playlist_backdrop(playlist));

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
                            .child("Плейлисты"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Ваши коллекции Яндекс Музыки"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .when(!items.is_empty(), |element| {
                        element.child(
                            Badge::new(format::plural(
                                items.len() as u64,
                                "плейлист",
                                "плейлиста",
                                "плейлистов",
                            ))
                            .tone(BadgeTone::Neutral),
                        )
                    })
                    .child(
                        IconButton::new("refresh-playlists", icons::REFRESH)
                            .size(38.0)
                            .icon_size(16.0)
                            .variant(ButtonVariant::Secondary)
                            .disabled(loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.load_playlists(cx);
                            })),
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if let Some(error) = error {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .child(error_banner(theme, &error))
                    .child(
                        div().flex().justify_center().child(
                            Button::new("retry-playlists", "Повторить")
                                .variant(ButtonVariant::Secondary)
                                .icon(icons::REFRESH)
                                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.load_playlists(cx);
                                })),
                        ),
                    ),
            );
        }

        if loading && items.is_empty() {
            body = body.child(
                glass_card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем плейлисты…"),
                    ),
            );
        } else if !loaded && items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Плейлисты не загружены",
                "Обновите список, чтобы увидеть ваши коллекции.",
            ));
        } else if items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Пока нет плейлистов",
                "Создайте плейлист в Яндекс Музыке — он появится здесь.",
            ));
        } else {
            let columns = if self.viewport_width < 1100.0 { 2 } else { 3 };
            let mut grid = div().w_full().grid().grid_cols(columns).gap(px(space::LG));

            for playlist in &items {
                let id = playlist.playlist_id();
                let cover = self.playlist_cover_image(playlist);
                let title = playlist.display_title().to_string();
                let count = playlist.track_count.unwrap_or(playlist.tracks.len() as u64);
                let visibility = if playlist.is_public() {
                    "Публичный"
                } else {
                    "Приватный"
                };
                let playlist_for_click = playlist.clone();

                let card = div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "playlist-card-{id}"
                    ))))
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .p(px(space::SM))
                    .rounded(px(14.0))
                    .bg(gpui::rgba(0x0808088C))
                    .border_1()
                    .border_color(theme.colors.border)
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(theme.colors.surface_hover)
                            .border_color(theme.colors.accent)
                    })
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.open_playlist(playlist_for_click.clone(), cx);
                    }))
                    .child(playlist_cover(theme, cover, 150.0))
                    .child(
                        div()
                            .px(px(space::XS))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
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
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(space::SM))
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(format::plural(count, "трек", "трека", "треков"))
                                    .child(visibility.to_string()),
                            ),
                    );

                grid = grid.child(card);
            }

            body = body.child(grid);
        }

        immersive_scroll_page(
            palette,
            backdrop,
            "playlists-scroll",
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
                        .child(body)
                        .child(
                            div()
                                .pt(px(space::SM))
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Плейлисты загружаются из вашей медиатеки Яндекс Музыки."),
                        ),
                )
                .into_any_element(),
        )
    }

    pub(crate) fn playlist_detail_content(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(playlist) = self.playlists.selected.clone() else {
            return div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child("Плейлист не выбран")
                .into_any_element();
        };

        let cover = self.playlist_cover_image(&playlist);
        let palette = self.playlist_palette(&playlist);
        let backdrop = self.playlist_backdrop(&playlist);
        let title = playlist.display_title().to_string();
        let owner = playlist
            .owner
            .as_ref()
            .and_then(|owner| {
                owner
                    .display_name
                    .as_deref()
                    .or(owner.full_name.as_deref())
                    .or(owner.login.as_deref())
            })
            .unwrap_or("Яндекс Музыка")
            .to_string();
        let is_public = playlist.is_public();
        let total = playlist
            .track_count
            .unwrap_or(self.playlists.tracks.len() as u64);
        let duration = playlist.duration();
        let tracks = self.playlists.tracks.clone();
        let loading = self.playlists.tracks_loading;
        let error = self.playlists.tracks_error.clone();
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-to-playlists", "К плейлистам")
                    .variant(ButtonVariant::Ghost)
                    .icon(icons::CHEVRON_LEFT)
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.back_to_playlists(cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        Button::new("play-playlist", "Слушать")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::PLAY)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.play_playlist(cx);
                            })),
                    )
                    .child(
                        Button::new("shuffle-playlist", "Перемешать")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::SHUFFLE)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.shuffle_playlist(cx);
                            })),
                    ),
            );

        let hero = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(space::XL))
            .p(px(space::XL))
            .rounded(px(20.0))
            .bg(gpui::rgba(0x0808088C))
            .border_1()
            .border_color(theme.colors.border)
            .child(playlist_cover(theme, cover, 168.0).w(px(168.0)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(
                                div()
                                    .text_size(px(30.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(px(34.0))
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(title),
                            )
                            .child(
                                Badge::new(if is_public {
                                    "Публичный"
                                } else {
                                    "Приватный"
                                })
                                .tone(if is_public {
                                    BadgeTone::Outline
                                } else {
                                    BadgeTone::Neutral
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child(owner),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::XL))
                            .pt(px(space::SM))
                            .child(
                                Stat::new(
                                    format::plural(total, "трек", "трека", "треков"),
                                    "в плейлисте",
                                )
                                .accent(true),
                            )
                            .child(divider_vertical(theme))
                            .child(Stat::new(format::duration(duration), "длительность")),
                    )
                    .when(
                        playlist
                            .description
                            .as_deref()
                            .is_some_and(|description| !description.is_empty()),
                        |element| {
                            element.child(
                                div()
                                    .text_size(px(text::SM))
                                    .text_color(theme.colors.text_muted)
                                    .max_w(px(560.0))
                                    .child(playlist.description.clone().unwrap_or_default()),
                            )
                        },
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if let Some(error) = error {
            body = body.child(error_banner(theme, &error));
        }

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
                            .child("Загружаем треки плейлиста…"),
                    ),
            );
        } else if tracks.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Плейлист пуст",
                "Добавьте треки в плейлист в Яндекс Музыке и обновите страницу.",
            ));
        } else {
            let mut list = glass_card(theme).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(section_title(theme, "Треки")),
                    )
                    .child(
                        Badge::new(format::plural(total, "трек", "трека", "треков"))
                            .tone(BadgeTone::Neutral),
                    ),
            );

            for (index, track) in tracks.iter().enumerate() {
                let is_current = current_id.as_deref() == Some(track.id.as_str());
                let track_cover = self.covers.get(&track.id).cloned();
                let row = TrackRow::new(
                    ElementId::Name(SharedString::from(format!(
                        "playlist-track-{}-{index}",
                        track.id
                    ))),
                    index + 1,
                    track.display_title().to_string(),
                    track.artist_names(),
                    format::duration_or_dash(track.duration()),
                )
                .cover(track_cover)
                .current(is_current, playing)
                .animation_phase(self.animation_phase)
                .explicit(track.explicit.unwrap_or(false))
                .on_play(cx.listener({
                    let track = track.clone();
                    move |this, _: &gpui::ClickEvent, _, cx| {
                        this.play_playlist_track(track.clone(), cx);
                    }
                }));

                let row = track_navigation(row, track, cx);
                list = list.child(row);
            }

            if total as usize > tracks.len() {
                list = list.child(
                    div()
                        .pt(px(space::SM))
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child(format!("Показано {} из {} треков", tracks.len(), total)),
                );
            }

            body = body.child(list);
        }

        if self.playlists.detail_loading_more {
            body = body.child(
                div()
                    .flex()
                    .justify_center()
                    .py(px(space::MD))
                    .child(Spinner::new().size(18.0).color(theme.colors.accent)),
            );
        }

        immersive_scroll_page_tracked(
            palette,
            backdrop,
            "playlist-detail-scroll",
            &self.playlist_scroll,
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

    pub(crate) fn playlist_palette(&self, playlist: &Playlist) -> (Rgba, Rgba) {
        let key = format!("playlist-{}", playlist.playlist_id());
        self.track_palettes
            .get(&key)
            .copied()
            .or_else(|| {
                playlist
                    .tracks
                    .first()
                    .and_then(|short| self.track_palettes.get(&short.id).copied())
            })
            .unwrap_or_else(default_palette)
    }

    pub(crate) fn playlist_backdrop(&self, playlist: &Playlist) -> Option<Arc<Image>> {
        let key = format!("playlist-{}", playlist.playlist_id());
        self.track_backdrops.get(&key).cloned().or_else(|| {
            playlist
                .tracks
                .first()
                .and_then(|short| self.track_backdrops.get(&short.id).cloned())
        })
    }

    pub(crate) fn playlist_cover_image(&self, playlist: &Playlist) -> Option<Arc<Image>> {
        self.playlist_covers
            .get(&playlist.playlist_id())
            .cloned()
            .or_else(|| {
                playlist
                    .tracks
                    .first()
                    .and_then(|short| self.covers.get(&short.id).cloned())
            })
    }
}
