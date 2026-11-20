//! Album page rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn album_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let content = self.album_content(theme, cx);
        self.app_shell(theme, cx, content)
    }

    pub(crate) fn album_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let album = self.album.album.clone();
        let tracks = self.album.tracks.clone();
        let loading = self.album.loading;
        let error = self.album.error.clone();
        let album_id = self.album.id;
        let background_key = album_id.map(|id| format!("album-{id}")).unwrap_or_default();
        let palette = self.track_palette(&background_key);
        let backdrop = self.track_backdrop(&background_key);
        let cover = album_id.and_then(|id| self.search_images.get(&format!("album-{id}")).cloned());
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let title = album
            .as_ref()
            .map(|album| album.display_title().to_string())
            .unwrap_or_else(|| "Альбом".to_string());
        let artists = album
            .as_ref()
            .map(|album| album.artists.clone())
            .unwrap_or_default();
        let year = album.as_ref().and_then(|album| album.year);
        let track_count = album
            .as_ref()
            .and_then(|album| album.track_count)
            .filter(|count| *count > 0)
            .unwrap_or(tracks.len() as u64);
        let duration = album
            .as_ref()
            .map(|album| album.duration())
            .unwrap_or_default();
        let description = album
            .as_ref()
            .and_then(|album| album.short_description.clone());

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-from-album", "Назад")
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
                        Button::new("album-play", "Слушать")
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
                        Button::new("album-shuffle", "Перемешать")
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

        let mut artist_row = div().flex().items_center().gap(px(space::XS)).flex_wrap();
        for (index, artist) in artists.iter().enumerate() {
            let Some(id) = artist.id else {
                continue;
            };
            let artist_for_click = artist.clone();
            artist_row = artist_row.child(
                Button::new(
                    ElementId::Name(SharedString::from(format!("album-artist-{id}-{index}"))),
                    artist.display_name().to_string(),
                )
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.open_artist(id, Some(artist_for_click.clone()), cx);
                })),
            );
        }

        let hero = glass_card(theme)
            .flex_row()
            .items_center()
            .gap(px(space::XL))
            .p(px(space::XL))
            .child(search_thumb(theme, cover, icons::MUSIC, 168.0, 16.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .text_size(px(32.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .line_height(px(36.0))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(title),
                    )
                    .when(!artists.is_empty(), |element| element.child(artist_row))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::XL))
                            .pt(px(space::SM))
                            .when_some(year, |element, year| {
                                element
                                    .child(Stat::new(year.to_string(), "год"))
                                    .child(divider_vertical(theme))
                            })
                            .child(Stat::new(
                                format::plural(track_count, "трек", "трека", "треков"),
                                "в альбоме",
                            ))
                            .child(divider_vertical(theme))
                            .child(Stat::new(format::duration(duration), "длительность")),
                    )
                    .when_some(description, |element, description| {
                        element.child(
                            div()
                                .max_w(px(620.0))
                                .text_size(px(text::SM))
                                .text_color(theme.colors.text_muted)
                                .child(description),
                        )
                    }),
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
                            .child("Загружаем альбом…"),
                    ),
            );
        } else {
            if let Some(error) = error {
                body = body.child(error_banner(theme, &error));
            }

            let mut list = glass_card(theme).child(
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
                list = list.child(
                    div()
                        .py(px(space::XL))
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child("В этом альбоме нет доступных треков."),
                );
            } else {
                let context = tracks.clone();
                for (index, track) in tracks.iter().enumerate() {
                    let is_current = current_id.as_deref() == Some(track.id.as_str());
                    let cover = self.covers.get(&track.id).cloned();
                    let row = TrackRow::new(
                        ElementId::Name(SharedString::from(format!(
                            "album-track-{}-{index}",
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
                    list = list.child(row);
                }
            }

            body = body.child(list);
        }

        immersive_scroll_page(
            palette,
            backdrop,
            "album-scroll",
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
}
