use crate::theme::{Theme, icons, space, text};
use gpui::{
    App, Bounds, ClickEvent, ElementId, FontWeight, Image, ImageSource, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, RenderOnce, SharedString, Window,
    canvas, img, prelude::*, px, relative,
};
use std::sync::{Arc, Mutex};

use super::{Badge, BadgeTone, ButtonVariant, Icon, IconButton, Slider};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type ArtistHandler = Arc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static>;
type TrackHandler = Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type ControlsHoverHandler = Arc<dyn Fn(bool, &mut Window, &mut App) + 'static>;
type SeekHandler = Arc<dyn Fn(f32, &mut Window, &mut App) + 'static>;
type SeekHoverHandler = Arc<dyn Fn(Option<f32>, &mut Window, &mut App) + 'static>;
type VolumeHandler = Box<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// The persistent playback bar at the bottom of the window.
#[derive(IntoElement)]
pub struct PlayerBar {
    title: SharedString,
    artist: SharedString,
    artist_names: Vec<SharedString>,
    cover: Option<Arc<Image>>,
    progress: f32,
    elapsed: SharedString,
    duration: SharedString,
    playing: bool,
    loading: bool,
    quality: Option<SharedString>,
    error: Option<SharedString>,
    volume: f32,
    compact: bool,
    liked: bool,
    explicit: bool,
    queue_open: bool,
    on_like: Option<ClickHandler>,
    on_queue: Option<ClickHandler>,
    on_track: Option<TrackHandler>,
    on_artist: Option<ArtistHandler>,
    on_mute: Option<ClickHandler>,
    on_volume: Option<VolumeHandler>,
    on_toggle: Option<ClickHandler>,
    on_previous: Option<ClickHandler>,
    on_next: Option<ClickHandler>,
    on_seek: Option<SeekHandler>,
    on_seek_preview: Option<SeekHandler>,
    on_seek_hover: Option<SeekHoverHandler>,
    on_controls_hover: Option<ControlsHoverHandler>,
    controls_hovered: bool,
    seek_hover: Option<f32>,
    seek_preview: Option<f32>,
    seek_time: Option<SharedString>,
}

impl PlayerBar {
    pub fn new() -> Self {
        Self {
            title: "".into(),
            artist: "".into(),
            artist_names: Vec::new(),
            cover: None,
            progress: 0.0,
            elapsed: "0:00".into(),
            duration: "0:00".into(),
            playing: false,
            loading: false,
            quality: None,
            error: None,
            volume: 0.85,
            compact: false,
            liked: false,
            explicit: false,
            queue_open: false,
            on_like: None,
            on_queue: None,
            on_track: None,
            on_artist: None,
            on_mute: None,
            on_volume: None,
            on_toggle: None,
            on_previous: None,
            on_next: None,
            on_seek: None,
            on_seek_preview: None,
            on_seek_hover: None,
            on_controls_hover: None,
            controls_hovered: false,
            seek_hover: None,
            seek_preview: None,
            seek_time: None,
        }
    }

    pub fn track(
        mut self,
        title: impl Into<SharedString>,
        artist: impl Into<SharedString>,
    ) -> Self {
        self.title = title.into();
        self.artist = artist.into();
        self
    }

    pub fn explicit(mut self, explicit: bool) -> Self {
        self.explicit = explicit;
        self
    }

    pub fn cover(mut self, cover: Option<Arc<Image>>) -> Self {
        self.cover = cover;
        self
    }

    pub fn progress(
        mut self,
        progress: f32,
        elapsed: impl Into<SharedString>,
        duration: impl Into<SharedString>,
    ) -> Self {
        self.progress = progress;
        self.elapsed = elapsed.into();
        self.duration = duration.into();
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn quality(mut self, quality: Option<impl Into<SharedString>>) -> Self {
        self.quality = quality.map(Into::into);
        self
    }

    pub fn error(mut self, error: Option<impl Into<SharedString>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Hide non-essential controls in a narrow window.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn liked(mut self, liked: bool) -> Self {
        self.liked = liked;
        self
    }

    pub fn on_like(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_like = Some(Box::new(handler));
        self
    }

    pub fn queue_open(mut self, open: bool) -> Self {
        self.queue_open = open;
        self
    }

    pub fn on_queue(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_queue = Some(Box::new(handler));
        self
    }

    pub fn artists(mut self, artists: Vec<SharedString>) -> Self {
        self.artist_names = artists;
        self
    }

    pub fn on_artist(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_artist = Some(Arc::new(handler));
        self
    }

    pub fn on_track(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_track = Some(Arc::new(handler));
        self
    }

    pub fn on_mute(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mute = Some(Box::new(handler));
        self
    }

    pub fn on_volume(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_volume = Some(Box::new(handler));
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    pub fn on_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_next = Some(Box::new(handler));
        self
    }

    pub fn on_previous(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_previous = Some(Box::new(handler));
        self
    }

    pub fn on_seek(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Arc::new(handler));
        self
    }

    pub fn on_seek_preview(
        mut self,
        handler: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_seek_preview = Some(Arc::new(handler));
        self
    }

    pub fn on_seek_hover(
        mut self,
        handler: impl Fn(Option<f32>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_seek_hover = Some(Arc::new(handler));
        self
    }

    pub fn on_controls_hover(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_controls_hover = Some(Arc::new(handler));
        self
    }

    pub fn controls_hovered(mut self, hovered: bool) -> Self {
        self.controls_hovered = hovered;
        self
    }

    pub fn seek_hover(mut self, fraction: Option<f32>) -> Self {
        self.seek_hover = fraction;
        self
    }

    pub fn seek_preview(mut self, fraction: Option<f32>) -> Self {
        self.seek_preview = fraction;
        self
    }

    pub fn seek_time(mut self, time: Option<SharedString>) -> Self {
        self.seek_time = time;
        self
    }
}

fn seek_surface(
    on_hover: Option<SeekHoverHandler>,
    on_preview: Option<SeekHandler>,
    on_commit: Option<SeekHandler>,
) -> gpui::Stateful<gpui::Div> {
    let bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> = Arc::new(Mutex::new(None));
    let bounds_for_canvas = bounds.clone();
    let bounds_for_move = bounds.clone();
    let bounds_for_down = bounds.clone();
    let bounds_for_up = bounds.clone();
    let bounds_for_out = bounds;
    let hover_for_leave = on_hover.clone();
    let hover_for_move = on_hover;
    let preview_for_down = on_preview.clone();
    let preview_for_move = on_preview;
    let commit_for_up = on_commit.clone();
    let commit_for_out = on_commit;

    gpui::div()
        .id("player-seek-surface")
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .cursor_pointer()
        .on_hover(move |hovered, window, cx| {
            if !hovered && let Some(handler) = hover_for_leave.as_ref() {
                handler(None, window, cx);
            }
        })
        .on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                let Ok(bounds) = bounds_for_down.lock() else {
                    return;
                };
                let Some(fraction) = seek_fraction(event.position.x, bounds.as_ref()) else {
                    return;
                };
                if let Some(handler) = preview_for_down.as_ref() {
                    handler(fraction, window, cx);
                }
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
            let Ok(bounds) = bounds_for_move.lock() else {
                return;
            };
            let Some(fraction) = seek_fraction(event.position.x, bounds.as_ref()) else {
                return;
            };
            if let Some(handler) = hover_for_move.as_ref() {
                handler(Some(fraction), window, cx);
            }
            if event.dragging()
                && let Some(handler) = preview_for_move.as_ref()
            {
                handler(fraction, window, cx);
            }
        })
        .on_mouse_up(
            MouseButton::Left,
            move |event: &MouseUpEvent, window, cx| {
                let Ok(bounds) = bounds_for_up.lock() else {
                    return;
                };
                let Some(fraction) = seek_fraction(event.position.x, bounds.as_ref()) else {
                    return;
                };
                if let Some(handler) = commit_for_up.as_ref() {
                    handler(fraction, window, cx);
                }
            },
        )
        .on_mouse_up_out(
            MouseButton::Left,
            move |event: &MouseUpEvent, window, cx| {
                let Ok(bounds) = bounds_for_out.lock() else {
                    return;
                };
                let Some(fraction) = seek_fraction(event.position.x, bounds.as_ref()) else {
                    return;
                };
                if let Some(handler) = commit_for_out.as_ref() {
                    handler(fraction, window, cx);
                }
            },
        )
        .child(
            canvas(
                move |bounds, _, _| {
                    if let Ok(mut slot) = bounds_for_canvas.lock() {
                        *slot = Some(bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        )
}

fn clear_tooltip_on_hover(
    button: IconButton,
    seek_handler: Option<SeekHoverHandler>,
    controls_handler: Option<ControlsHoverHandler>,
) -> IconButton {
    button.on_hover(move |hovered, window, cx| {
        if let Some(handler) = controls_handler.as_ref() {
            handler(*hovered, window, cx);
        }
        if *hovered && let Some(handler) = seek_handler.as_ref() {
            handler(None, window, cx);
        }
    })
}

fn seek_fraction(event_x: gpui::Pixels, bounds: Option<&Bounds<gpui::Pixels>>) -> Option<f32> {
    let bounds = bounds?;
    let width = f32::from(bounds.size.width);
    if width <= 0.0 {
        return None;
    }
    Some(((f32::from(event_x) - f32::from(bounds.origin.x)) / width).clamp(0.0, 1.0))
}

impl Default for PlayerBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for PlayerBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let has_track = !self.title.is_empty();

        let cover_content = match self.cover {
            Some(image) => img(ImageSource::Image(image))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => gpui::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icons::MUSIC)
                        .size(20.0)
                        .color(theme.colors.text_muted),
                )
                .into_any_element(),
        };

        let title = if has_track {
            self.title.clone()
        } else {
            SharedString::from("Ничего не играет")
        };
        let subtitle = if let Some(error) = &self.error {
            error.clone()
        } else if has_track {
            self.artist.clone()
        } else {
            SharedString::from("Выберите трек из коллекции")
        };
        let subtitle_color = if self.error.is_some() {
            theme.colors.danger
        } else {
            theme.colors.text_muted
        };

        let track_handler = self.on_track.clone();

        let left_meta = gpui::div()
            .flex()
            .flex_col()
            .min_w(px(0.0))
            .gap(px(2.0))
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .min_w(px(0.0))
                    .child(
                        gpui::div()
                            .id("player-track-title")
                            .flex_shrink()
                            .min_w(px(0.0))
                            .text_size(px(text::BASE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .border_b_1()
                            .border_color(gpui::transparent_black())
                            .hover(|style| style.border_color(theme.colors.accent))
                            .child(title),
                    )
                    .when(self.explicit, |element| {
                        element.child(
                            Icon::new(icons::EXPLICIT)
                                .size(12.0)
                                .color(theme.colors.text_muted),
                        )
                    }),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap(px(space::XS))
                    .text_size(px(text::SM))
                    .text_color(subtitle_color)
                    .when(self.error.is_some(), |element| {
                        element.child(
                            Icon::new(icons::WARNING)
                                .size(12.0)
                                .color(theme.colors.danger),
                        )
                    })
                    .child({
                        let mut artist_row = gpui::div()
                            .id("player-artist")
                            .flex()
                            .items_center()
                            .gap(px(space::XS))
                            .w_full()
                            .min_w(px(0.0))
                            .overflow_hidden();

                        if self.error.is_some() {
                            artist_row = artist_row.child(
                                Icon::new(icons::WARNING)
                                    .size(12.0)
                                    .color(theme.colors.danger),
                            );
                        }

                        if has_track && self.error.is_none() && !self.artist_names.is_empty() {
                            let handler = self.on_artist.clone();
                            let mut links = gpui::div()
                                .flex()
                                .items_center()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden();
                            for (index, name) in self.artist_names.iter().enumerate() {
                                if index > 0 {
                                    links = links.child(
                                        gpui::div().text_color(theme.colors.text_muted).child(", "),
                                    );
                                }
                                let mut link = gpui::div()
                                    .id(ElementId::Name(SharedString::from(format!(
                                        "player-artist-{index}"
                                    ))))
                                    .cursor_pointer()
                                    .child(
                                        gpui::div()
                                            .border_b_1()
                                            .border_color(gpui::transparent_black())
                                            .whitespace_nowrap()
                                            .hover(|style| {
                                                style
                                                    .text_color(theme.colors.accent)
                                                    .border_color(theme.colors.accent)
                                            })
                                            .child(name.clone()),
                                    );
                                if let Some(handler) = handler.clone() {
                                    link = link.on_click(move |event, window, cx| {
                                        cx.stop_propagation();
                                        handler(index, event, window, cx);
                                    });
                                }
                                links = links.child(link);
                            }
                            artist_row = artist_row.child(links);
                        } else {
                            artist_row = artist_row.child(
                                gpui::div()
                                    .min_w(px(0.0))
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(subtitle),
                            );
                        }
                        artist_row
                    }),
            );

        let mut left = gpui::div()
            .id("player-track-info")
            .relative()
            .flex()
            .items_center()
            .gap(px(space::MD))
            .w(px(if self.compact { 260.0 } else { 300.0 }))
            .h_full()
            .flex_shrink_0();

        left = left
            .child(
                gpui::div()
                    .size(px(52.0))
                    .rounded(px(8.0))
                    .overflow_hidden()
                    .bg(theme.colors.surface_active)
                    .flex_shrink_0()
                    .child(cover_content),
            )
            .child(left_meta);

        if has_track && let Some(handler) = track_handler {
            left = left.cursor_pointer().on_click(move |event, window, cx| {
                handler(event, window, cx);
            });
        }

        let tooltip_hover = self.on_seek_hover.clone();
        let controls_hover = self.on_controls_hover.clone();
        let mut toggle = IconButton::new(
            "player-toggle",
            if self.playing {
                icons::PAUSE
            } else {
                icons::PLAY
            },
        )
        .size(42.0)
        .icon_size(18.0)
        .variant(ButtonVariant::Primary)
        .loading(self.loading);
        toggle = clear_tooltip_on_hover(toggle, tooltip_hover.clone(), controls_hover.clone());

        if !has_track || self.loading {
            toggle = toggle.disabled(true);
        } else if let Some(handler) = self.on_toggle {
            toggle = toggle.on_click(handler);
        }

        let mut previous = IconButton::new("player-previous", icons::PREVIOUS)
            .size(36.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost);
        previous = clear_tooltip_on_hover(previous, tooltip_hover.clone(), controls_hover.clone());
        if !has_track || self.loading {
            previous = previous.disabled(true);
        } else if let Some(handler) = self.on_previous {
            previous = previous.on_click(handler);
        }

        let mut next = IconButton::new("player-next", icons::NEXT)
            .size(36.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost);
        next = clear_tooltip_on_hover(next, tooltip_hover.clone(), controls_hover.clone());
        if !has_track || self.loading {
            next = next.disabled(true);
        } else if let Some(handler) = self.on_next {
            next = next.on_click(handler);
        }

        let mut like = IconButton::new(
            "player-like",
            if self.liked {
                icons::HEART
            } else {
                icons::HEART_OUTLINE
            },
        )
        .size(34.0)
        .icon_size(16.0)
        .variant(if self.liked {
            ButtonVariant::Soft
        } else {
            ButtonVariant::Ghost
        });
        like = clear_tooltip_on_hover(like, tooltip_hover.clone(), controls_hover.clone());
        if !has_track || self.loading {
            like = like.disabled(true);
        } else if let Some(handler) = self.on_like {
            like = like.on_click(handler);
        }

        let mut queue = IconButton::new("player-queue", icons::QUEUE)
            .size(34.0)
            .icon_size(16.0)
            .variant(if self.queue_open {
                ButtonVariant::Soft
            } else {
                ButtonVariant::Ghost
            });
        if let Some(handler) = self.on_queue {
            queue = queue.on_click(handler);
        }

        let controls = gpui::div()
            .id("player-controls")
            .occlude()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .child(gpui::div().w(px(76.0)).flex_shrink_0())
            .child(previous)
            .child(toggle)
            .child(next)
            .child(like)
            .child(queue);

        let progress_row = gpui::div()
            .w_full()
            .max_w(px(520.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                gpui::div()
                    .w(px(38.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .text_right()
                    .child(self.elapsed),
            )
            .child(
                gpui::div()
                    .w(px(38.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child(self.duration),
            );

        let seek_active = self.seek_hover.is_some() || self.seek_preview.is_some();
        let mut center = gpui::div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .rounded(px(12.0))
            .bg(gpui::rgba(0xFFFFFF06))
            .hover(|style| style.bg(gpui::rgba(0xFFFFFF0F)));

        if seek_active {
            center = center.bg(gpui::rgba(0xFFFFFF0D));
        }

        center = center.child(
            gpui::div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full()
                .rounded(px(12.0))
                .overflow_hidden()
                .child(
                    gpui::div()
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .h_full()
                        .w(relative(self.progress))
                        .bg(if seek_active {
                            gpui::rgba(0xFFFFFF33)
                        } else {
                            gpui::rgba(0xFFFFFF1F)
                        }),
                ),
        );

        center = center.child(seek_surface(
            self.on_seek_hover,
            self.on_seek_preview,
            self.on_seek,
        ));

        center = center
            .child(controls)
            .child(progress_row)
            .child(
                gpui::div()
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .w(px(1.0))
                    .bg(theme.colors.border),
            )
            .child(
                gpui::div()
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .right(px(0.0))
                    .w(px(1.0))
                    .bg(theme.colors.border),
            );

        if let Some(time) = self.seek_time.filter(|_| !self.controls_hovered) {
            let fraction = self.seek_preview.unwrap_or(self.progress).clamp(0.0, 0.88);
            center = center.child(
                gpui::div()
                    .absolute()
                    .top(px(-28.0))
                    .left(relative(fraction))
                    .rounded(px(999.0))
                    .bg(gpui::rgba(0x111111F2))
                    .border_1()
                    .border_color(gpui::rgba(0xFFFFFF24))
                    .px(px(space::SM))
                    .py(px(3.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text)
                    .whitespace_nowrap()
                    .child(time),
            );
        }

        let compact = self.compact;
        let volume = self.volume;
        let muted = volume <= 0.001;
        let mut mute = IconButton::new(
            "player-mute",
            if muted {
                icons::VOLUME_MUTED
            } else {
                icons::VOLUME
            },
        )
        .size(30.0)
        .icon_size(15.0);
        mute = clear_tooltip_on_hover(mute, tooltip_hover, controls_hover);
        if let Some(handler) = self.on_mute {
            mute = mute.on_click(handler);
        }

        let mut slider = Slider::new(volume).width(if compact { 56.0 } else { 72.0 });
        if let Some(handler) = self.on_volume {
            slider = slider.on_change(handler);
        }

        let volume_group = gpui::div()
            .flex()
            .items_center()
            .gap(px(space::XS))
            .child(mute)
            .child(slider)
            .child(
                gpui::div()
                    .w(px(30.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .text_right()
                    .child(format!("{:.0}%", volume * 100.0)),
            );

        let mut right = gpui::div()
            .w(px(if compact { 146.0 } else { 300.0 }))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(space::SM))
            .child(volume_group);

        if !compact {
            if self.error.is_some() {
                right = right.child(Badge::new("Ошибка").tone(BadgeTone::Danger));
            }

            if let Some(quality) = self.quality {
                right = right.child(
                    Badge::new(quality)
                        .tone(BadgeTone::Outline)
                        .icon(icons::MUSIC),
                );
            }
        }

        gpui::div()
            .id("player-bar")
            .w_full()
            .h(px(88.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(space::XL))
            .px(px(space::XL))
            .bg(theme.colors.surface)
            .border_t_1()
            .border_color(theme.colors.border)
            .child(left)
            .child(center)
            .child(right)
    }
}
