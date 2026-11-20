//! Queue drawer rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn queue_drawer(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let (start, end) = self.queue_window();
        let total = self.queue.len();
        let current_index = self.queue_index;
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;
        let phase = self.animation_phase as usize;
        let width = if self.viewport_width < 1100.0 {
            300.0
        } else {
            360.0
        };

        let mut list = div().flex().flex_col().gap(px(2.0));

        for (offset, id) in self.queue[start..end].iter().enumerate() {
            let index = start + offset;
            let track = self.queue_track_cache.get(id).cloned();
            let current = current_index == Some(index)
                || track
                    .as_ref()
                    .is_some_and(|track| current_id.as_deref() == Some(track.id.as_str()));

            let leading = if current {
                div()
                    .w(px(20.0))
                    .flex()
                    .items_end()
                    .justify_center()
                    .gap(px(2.0))
                    .child(
                        div()
                            .w(px(3.0))
                            .h(px(5.0 + ((phase + index) % 4) as f32 * 2.0))
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .child(
                        div()
                            .w(px(3.0))
                            .h(px(12.0 - ((phase + index + 1) % 3) as f32 * 2.0))
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .child(
                        div()
                            .w(px(3.0))
                            .h(if playing {
                                px(8.0 + ((phase + index + 2) % 3) as f32 * 2.0)
                            } else {
                                px(5.0)
                            })
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .into_any_element()
            } else {
                div()
                    .w(px(20.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .text_right()
                    .child(format!("{}", index + 1))
                    .into_any_element()
            };

            let title = track
                .as_ref()
                .map(|track| track.display_title().to_string())
                .unwrap_or_else(|| "Загрузка…".to_string());
            let artist = track
                .as_ref()
                .map(|track| track.artist_names())
                .unwrap_or_default();
            let duration = track
                .as_ref()
                .map(|track| format::duration_or_dash(track.duration()))
                .unwrap_or_default();
            let explicit = track
                .as_ref()
                .is_some_and(|track| track.explicit.unwrap_or(false));

            let row = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "queue-row-{index}"
                ))))
                .flex()
                .items_center()
                .gap(px(space::SM))
                .h(px(50.0))
                .px(px(space::SM))
                .rounded(px(8.0))
                .cursor_pointer()
                .when(current, |element| element.bg(theme.colors.accent_soft))
                .hover(|style| style.bg(theme.colors.surface_hover))
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.play_queue_at(index, cx);
                }))
                .child(leading)
                .child({
                    let mut title_element = div()
                        .id(ElementId::Name(SharedString::from(format!(
                            "queue-title-{index}"
                        ))))
                        .flex_shrink()
                        .min_w(px(0.0))
                        .text_size(px(text::SM))
                        .font_weight(if current {
                            gpui::FontWeight::SEMIBOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(if current {
                            theme.colors.accent
                        } else {
                            theme.colors.text
                        })
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(title);
                    if let Some(track) = track.clone() {
                        title_element = title_element
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.colors.accent).underline())
                            .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.open_track_from_queue(track.clone(), cx);
                            }));
                    }

                    let artist_group = SharedString::from(format!("queue-artist-hover-{index}"));
                    let mut artist_element = div()
                        .id(ElementId::Name(SharedString::from(format!(
                            "queue-artist-{index}"
                        ))))
                        .group(artist_group.clone())
                        .w_full()
                        .child(
                            div().flex().child(
                                div()
                                    .border_b_1()
                                    .border_color(gpui::transparent_black())
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .group_hover(artist_group, |style| {
                                        style
                                            .text_color(theme.colors.accent)
                                            .border_color(theme.colors.accent)
                                    })
                                    .child(artist),
                            ),
                        );
                    if let Some(artist) = track
                        .as_ref()
                        .and_then(|track| track.artists.first().cloned())
                        && let Some(artist_id) = artist.id
                    {
                        artist_element = artist_element.cursor_pointer().on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.open_artist(artist_id, Some(artist.clone()), cx);
                            },
                        ));
                    }

                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(1.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .min_w(px(0.0))
                                .child(title_element)
                                .when(explicit, |element| {
                                    element.child(
                                        Icon::new(icons::EXPLICIT)
                                            .size(12.0)
                                            .color(theme.colors.text_muted),
                                    )
                                }),
                        )
                        .child(artist_element)
                })
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .child(duration),
                );

            list = list.child(row);
        }

        let body = if self.queue.is_empty() {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(space::SM))
                .p(px(space::XL))
                .child(
                    Icon::new(icons::QUEUE)
                        .size(28.0)
                        .color(theme.colors.text_muted),
                )
                .child(
                    div()
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_secondary)
                        .child("Очередь пуста"),
                )
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child("Включите трек, и здесь появится очередь воспроизведения."),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(space::SM))
                .p(px(space::SM))
                .child(list)
                .when(total > end - start, |element| {
                    element.child(
                        div()
                            .pt(px(space::SM))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .text_center()
                            .child(format!("Показано {} из {total}", end - start)),
                    )
                })
                .into_any_element()
        };

        div()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .h_full()
            .w(px(width))
            .flex()
            .flex_col()
            .bg(theme.colors.surface)
            .border_l_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(space::LG))
                    .py(px(space::MD))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::SM))
                            .child(section_title(theme, "Очередь"))
                            .child(Badge::new(format!("{total}")).tone(BadgeTone::Neutral)),
                    )
                    .child(
                        IconButton::new("close-queue", icons::CLOSE)
                            .size(30.0)
                            .icon_size(14.0)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.close_queue(cx)
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .id("queue-scroll")
                    .overflow_y_scroll()
                    .child(body),
            )
            .into_any_element()
    }
}
