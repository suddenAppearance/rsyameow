//! Search page and search result rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn search_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.search_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    pub(crate) fn search_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let results = self.search.results.clone();
        let artists = self.search.artists.clone();
        let albums = self.search.albums.clone();
        let kind = self.search.kind;
        let total = self.search.total;
        let loading = self.search.loading;
        let searched = self.search.searched;
        let query = self.search.query.clone();
        let page = self.search.page;
        let per_page = self.search.per_page.max(1);
        let pages = ((total as u64).saturating_add(per_page - 1) / per_page).max(1);
        let error = self.search.error.clone();
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let shown = match kind {
            SearchKind::Tracks => results.len(),
            SearchKind::Artists => artists.len(),
            SearchKind::Albums => albums.len(),
        };
        let (result_title, noun_one, noun_few, noun_many) = match kind {
            SearchKind::Tracks => ("Результаты", "трек", "трека", "треков"),
            SearchKind::Artists => ("Исполнители", "исполнитель", "исполнителя", "исполнителей"),
            SearchKind::Albums => ("Альбомы", "альбом", "альбома", "альбомов"),
        };

        let header = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(px(text::DISPLAY))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Поиск"),
            )
            .child(
                div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_secondary)
                    .child("Треки, исполнители и альбомы по всему каталогу"),
            );

        let tab = |id: &'static str,
                   label: &'static str,
                   selected: bool,
                   kind: SearchKind,
                   cx: &mut Context<Self>| {
            Button::new(id, label)
                .variant(if selected {
                    ButtonVariant::Soft
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Small)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.set_search_kind(kind, cx);
                }))
        };

        let mut search_card = card(theme)
            .child(
                div().flex().items_center().gap(px(space::SM)).child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(self.search_input.clone()),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::XS))
                    .pt(px(space::XS))
                    .child(tab(
                        "search-tab-tracks",
                        "Треки",
                        kind == SearchKind::Tracks,
                        SearchKind::Tracks,
                        cx,
                    ))
                    .child(tab(
                        "search-tab-artists",
                        "Исполнители",
                        kind == SearchKind::Artists,
                        SearchKind::Artists,
                        cx,
                    ))
                    .child(tab(
                        "search-tab-albums",
                        "Альбомы",
                        kind == SearchKind::Albums,
                        SearchKind::Albums,
                        cx,
                    )),
            );

        if self.demo {
            search_card = search_card.child(
                div()
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child("В демо-режиме поиск работает по локальной коллекции."),
            );
        }

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if loading {
            body = body.child(
                card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("Ищем «{query}»…")),
                    ),
            );
        } else if !searched {
            let (title, detail) = match kind {
                SearchKind::Tracks => (
                    "Что будем слушать?",
                    "Введите название трека, исполнителя или альбома — поиск начнётся автоматически.",
                ),
                SearchKind::Artists => (
                    "Найдите исполнителя",
                    "Введите имя исполнителя, чтобы увидеть его в каталоге.",
                ),
                SearchKind::Albums => (
                    "Найдите альбом",
                    "Введите название альбома или исполнителя.",
                ),
            };
            body = body.child(empty_search_state(theme, icons::SEARCH, title, detail));
        } else {
            if let Some(error) = error {
                body = body.child(error_banner(theme, &error));
            } else if shown == 0 {
                body = body.child(empty_search_state(
                    theme,
                    icons::SEARCH,
                    "Ничего не найдено",
                    &format!(
                        "По запросу «{query}» ничего не нашлось. Попробуйте сформулировать иначе."
                    ),
                ));
            } else {
                let mut list = card(theme).child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(space::MD))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .child(section_title(theme, result_title))
                                .child(
                                    Badge::new(format::plural(
                                        total as u64,
                                        noun_one,
                                        noun_few,
                                        noun_many,
                                    ))
                                    .tone(BadgeTone::Neutral),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child(if total > shown {
                                    format!("Показано {shown} из {total}")
                                } else {
                                    String::new()
                                }),
                        ),
                );

                match kind {
                    SearchKind::Tracks => {
                        for (index, track) in results.iter().enumerate() {
                            let is_current = current_id.as_deref() == Some(track.id.as_str());
                            let cover = self.covers.get(&track.id).cloned();
                            let row = TrackRow::new(
                                ElementId::Name(SharedString::from(format!(
                                    "search-{}-{index}",
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
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_search_result(track.clone(), cx);
                                }
                            }));

                            let row = track_navigation(row, track, cx);
                            list = list.child(row);
                        }
                    }
                    SearchKind::Artists => {
                        for (index, artist) in artists.iter().enumerate() {
                            let id = artist.id.unwrap_or(index as u64);
                            let image = self.search_images.get(&format!("artist-{id}")).cloned();
                            let name = artist.display_name().to_string();
                            let genres = if artist.genres.is_empty() {
                                "Исполнитель".to_string()
                            } else {
                                artist.genres.join(", ")
                            };

                            let artist_for_click = artist.clone();
                            let mut row = div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "search-artist-{id}-{index}"
                                ))))
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .h(px(60.0))
                                .px(px(space::SM))
                                .rounded(px(10.0))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.colors.surface_hover))
                                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.open_artist(id, Some(artist_for_click.clone()), cx);
                                }))
                                .child(search_thumb(theme, image, icons::USER, 44.0, 10.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .gap(px(1.0))
                                        .child(
                                            div()
                                                .text_size(px(text::BASE))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(text::XS))
                                                .text_color(theme.colors.text_muted)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(genres),
                                        ),
                                )
                                .child(
                                    Icon::new(icons::CHEVRON_RIGHT)
                                        .size(15.0)
                                        .color(theme.colors.text_muted),
                                );

                            row = row.w_full();
                            list = list.child(row);
                        }
                    }
                    SearchKind::Albums => {
                        for (index, album) in albums.iter().enumerate() {
                            let id = album.id.unwrap_or(index as u64);
                            let image = self.search_images.get(&format!("album-{id}")).cloned();
                            let title = album.display_title().to_string();
                            let artists = album.artist_names();
                            let meta = match (artists.is_empty(), album.year) {
                                (false, Some(year)) => format!("{artists} · {year}"),
                                (false, None) => artists,
                                (true, Some(year)) => year.to_string(),
                                (true, None) => "Альбом".to_string(),
                            };
                            let album_for_click = album.clone();

                            let row = div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "search-album-{id}-{index}"
                                ))))
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .h(px(60.0))
                                .px(px(space::SM))
                                .rounded(px(10.0))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.colors.surface_hover))
                                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.open_album(id, Some(album_for_click.clone()), cx);
                                }))
                                .child(search_thumb(theme, image, icons::MUSIC, 44.0, 6.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_w(px(0.0))
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
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(meta),
                                        ),
                                )
                                .child(
                                    Icon::new(icons::CHEVRON_RIGHT)
                                        .size(15.0)
                                        .color(theme.colors.text_muted),
                                );

                            list = list.child(row);
                        }
                    }
                }

                if !self.demo && total > shown {
                    list = list.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(space::SM))
                            .pt(px(space::MD))
                            .child(
                                Button::new("search-prev", "Назад")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Small)
                                    .icon(icons::CHEVRON_LEFT)
                                    .disabled(page == 0 || loading)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.search_previous(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("Страница {} из {pages}", page + 1)),
                            )
                            .child(
                                Button::new("search-next", "Вперёд")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Small)
                                    .trailing_icon(icons::CHEVRON_RIGHT)
                                    .disabled(
                                        (page as u64 + 1) * per_page >= total as u64 || loading,
                                    )
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.search_next(cx);
                                    })),
                            ),
                    );
                }

                body = body.child(list);
            }
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("search-scroll")
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
                        .child(search_card)
                        .child(body)
                        .child(
                            div()
                                .pt(px(space::SM))
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Поиск выполняется через официальный API Яндекс Музыки."),
                        ),
                ),
            )
            .into_any_element()
    }
}
