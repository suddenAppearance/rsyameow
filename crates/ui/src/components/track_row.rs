use crate::theme::{Theme, icons, space, text};
use gpui::{
    App, ClickEvent, ElementId, FontWeight, Image, ImageSource, IntoElement, ObjectFit, RenderOnce,
    SharedString, Window, img, prelude::*, px,
};
use std::sync::Arc;

use super::{ClickHandler, Icon};

/// A single song row: index or equalizer, cover, title, artist and duration.
#[derive(IntoElement)]
pub struct TrackRow {
    id: ElementId,
    index: usize,
    title: SharedString,
    artist: SharedString,
    duration: SharedString,
    cover: Option<Arc<Image>>,
    current: bool,
    playing: bool,
    explicit: bool,
    phase: u8,
    on_play: Option<ClickHandler>,
    on_title: Option<ClickHandler>,
    on_artist: Option<ClickHandler>,
}

impl TrackRow {
    pub fn new(
        id: impl Into<ElementId>,
        index: usize,
        title: impl Into<SharedString>,
        artist: impl Into<SharedString>,
        duration: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            index,
            title: title.into(),
            artist: artist.into(),
            duration: duration.into(),
            cover: None,
            current: false,
            playing: false,
            explicit: false,
            phase: 0,
            on_play: None,
            on_title: None,
            on_artist: None,
        }
    }

    pub fn cover(mut self, cover: Option<Arc<Image>>) -> Self {
        self.cover = cover;
        self
    }

    pub fn current(mut self, current: bool, playing: bool) -> Self {
        self.current = current;
        self.playing = playing;
        self
    }

    pub fn explicit(mut self, explicit: bool) -> Self {
        self.explicit = explicit;
        self
    }

    pub fn animation_phase(mut self, phase: u8) -> Self {
        self.phase = phase;
        self
    }

    pub fn on_play(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_play = Some(Box::new(handler));
        self
    }

    pub fn on_title(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_title = Some(Box::new(handler));
        self
    }

    pub fn on_artist(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_artist = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for TrackRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);

        let cover = match self.cover {
            Some(image) => img(ImageSource::Image(image))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => gpui::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.colors.surface_active)
                .child(
                    Icon::new(icons::MUSIC)
                        .size(18.0)
                        .color(theme.colors.text_muted),
                )
                .into_any_element(),
        };

        let group = SharedString::from(format!("track-row-{}-{}", self.index, self.title));
        let leading = if self.current {
            let phase = self.phase as usize;
            let heights = [
                5.0 + ((phase + self.index) % 4) as f32 * 2.5,
                13.0 - ((phase + self.index + 1) % 3) as f32 * 2.0,
                8.0 + ((phase + self.index + 2) % 3) as f32 * 2.0,
            ];
            gpui::div()
                .w(px(22.0))
                .h(px(16.0))
                .flex()
                .items_end()
                .justify_center()
                .gap(px(2.0))
                .child(
                    gpui::div()
                        .w(px(3.0))
                        .h(px(heights[0]))
                        .rounded_full()
                        .bg(theme.colors.accent),
                )
                .child(
                    gpui::div()
                        .w(px(3.0))
                        .h(px(heights[1]))
                        .rounded_full()
                        .bg(theme.colors.accent),
                )
                .child(
                    gpui::div()
                        .w(px(3.0))
                        .h(px(heights[2]))
                        .rounded_full()
                        .bg(theme.colors.accent),
                )
                .into_any_element()
        } else {
            gpui::div()
                .relative()
                .w(px(22.0))
                .h(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    gpui::div()
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_muted)
                        .group_hover(group.clone(), |style| style.opacity(0.0))
                        .child(format!("{}", self.index)),
                )
                .child(
                    gpui::div()
                        .absolute()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .opacity(0.0)
                        .group_hover(group.clone(), |style| style.opacity(1.0))
                        .child(Icon::new(icons::PLAY).size(13.0).color(theme.colors.accent)),
                )
                .into_any_element()
        };

        let mut row = gpui::div()
            .id(self.id)
            .group(group)
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(56.0))
            .px(px(space::SM))
            .rounded(px(10.0))
            .cursor_pointer()
            .hover(|style| style.bg(theme.colors.surface_hover));

        if self.current {
            row = row.bg(theme.colors.surface);
        }

        if let Some(handler) = self.on_play {
            row = row.on_click(handler);
        }

        row.child(leading)
            .child(
                gpui::div()
                    .size(px(40.0))
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .flex_shrink_0()
                    .child(cover),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(1.0))
                    .child({
                        let mut title = gpui::div()
                            .id(ElementId::Name(SharedString::from(format!(
                                "track-title-{}-{}",
                                self.index, self.title
                            ))))
                            .flex_shrink()
                            .min_w(px(0.0))
                            .cursor_pointer()
                            .text_size(px(text::BASE))
                            .font_weight(if self.current {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if self.current {
                                theme.colors.accent
                            } else {
                                theme.colors.text
                            })
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .hover(|style| style.text_color(theme.colors.accent))
                            .child(self.title);
                        if let Some(handler) = self.on_title {
                            title = title.on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                handler(event, window, cx);
                            });
                        }
                        gpui::div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .gap(px(6.0))
                            .min_w(px(0.0))
                            .child(title)
                            .when(self.explicit, |element| {
                                element.child(
                                    Icon::new(icons::EXPLICIT)
                                        .size(13.0)
                                        .color(theme.colors.text_muted),
                                )
                            })
                    })
                    .child({
                        let mut artist = gpui::div()
                            .id(ElementId::Name(SharedString::from(format!(
                                "track-artist-{}-{}",
                                self.index, self.artist
                            ))))
                            .cursor_pointer()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_muted)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .hover(|style| style.text_color(theme.colors.accent))
                            .child(self.artist);
                        if let Some(handler) = self.on_artist {
                            artist = artist.on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                handler(event, window, cx);
                            });
                        }
                        artist
                    }),
            )
            .child(
                gpui::div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_muted)
                    .w(px(44.0))
                    .text_right()
                    .child(self.duration),
            )
    }
}
