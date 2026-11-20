//! Saved albums page.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn albums_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.albums_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn albums_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let items = self.albums.items.clone();
        let loading = self.albums.loading;
        let loaded = self.albums.loaded;
        let error = self.albums.error.clone();
        let palette = items
            .first()
            .map(|album| self.album_palette(album))
            .unwrap_or_else(default_palette);
        let backdrop = items.first().and_then(|album| self.album_backdrop(album));

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
                            .child("Мои альбомы"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Сохранённые альбомы из вашей коллекции"),
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
                                "альбом",
                                "альбома",
                                "альбомов",
                            ))
                            .tone(BadgeTone::Neutral),
                        )
                    })
                    .child(
                        IconButton::new("refresh-albums", icons::REFRESH)
                            .size(38.0)
                            .icon_size(16.0)
                            .variant(ButtonVariant::Secondary)
                            .loading(loading)
                            .disabled(loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.load_albums(cx);
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
                            Button::new("retry-albums", "Повторить")
                                .variant(ButtonVariant::Secondary)
                                .icon(icons::REFRESH)
                                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.load_albums(cx);
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
                            .child("Загружаем альбомы…"),
                    ),
            );
        } else if !loaded && items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::ALBUM,
                "Альбомы не загружены",
                "Обновите список, чтобы увидеть сохранённые альбомы.",
            ));
        } else if items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::ALBUM,
                "Пока нет альбомов",
                "Добавьте альбом в коллекцию Яндекс Музыки — он появится здесь.",
            ));
        } else {
            let columns = if self.viewport_width < 1100.0 { 2 } else { 3 };
            let mut grid = div().w_full().grid().grid_cols(columns).gap(px(space::LG));

            for album in &items {
                let Some(id) = album.id else {
                    continue;
                };
                let cover = self.album_cover_image(album);
                let title = album.display_title().to_string();
                let artists = album.artist_names();
                let year = album.year;
                let count = album
                    .track_count
                    .unwrap_or_else(|| album.volumes.iter().map(Vec::len).sum::<usize>() as u64);
                let meta = match (year, count) {
                    (Some(year), _) => format!(
                        "{year} · {}",
                        format::plural(count, "трек", "трека", "треков")
                    ),
                    (None, _) => format::plural(count, "трек", "трека", "треков"),
                };
                let album_for_click = album.clone();

                grid = grid.child(
                    div()
                        .id(ElementId::Name(SharedString::from(format!(
                            "album-card-{id}"
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
                            this.open_album(id, Some(album_for_click.clone()), cx);
                        }))
                        .child(album_cover(theme, cover, 150.0))
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
                                .when(!artists.is_empty(), |element| {
                                    element.child(
                                        div()
                                            .text_size(px(text::XS))
                                            .text_color(theme.colors.text_secondary)
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(artists),
                                    )
                                })
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
            "albums-scroll",
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
                        .child(body),
                )
                .into_any_element(),
        )
    }

    fn album_palette(&self, album: &Album) -> (Rgba, Rgba) {
        let key = album.id.map(|id| format!("album-{id}")).unwrap_or_default();
        self.track_palettes
            .get(&key)
            .copied()
            .or_else(|| {
                album
                    .volumes
                    .first()
                    .and_then(|volume| volume.first())
                    .and_then(|track| self.track_palettes.get(&track.id).copied())
            })
            .unwrap_or_else(default_palette)
    }

    fn album_backdrop(&self, album: &Album) -> Option<Arc<Image>> {
        let key = album.id.map(|id| format!("album-{id}")).unwrap_or_default();
        self.track_backdrops.get(&key).cloned().or_else(|| {
            album
                .volumes
                .first()
                .and_then(|volume| volume.first())
                .and_then(|track| self.track_backdrops.get(&track.id).cloned())
        })
    }

    fn album_cover_image(&self, album: &Album) -> Option<Arc<Image>> {
        let key = album.id.map(|id| format!("album-{id}")).unwrap_or_default();
        self.search_images.get(&key).cloned().or_else(|| {
            album
                .volumes
                .first()
                .and_then(|volume| volume.first())
                .and_then(|track| self.covers.get(&track.id).cloned())
        })
    }
}
