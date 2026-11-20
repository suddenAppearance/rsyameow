use crate::theme::{Theme, icons, space, text};
use gpui::{
    App, Bounds, ClickEvent, FontWeight, Image, ImageSource, IntoElement, MouseButton,
    MouseDownEvent, ObjectFit, RenderOnce, SharedString, Window, canvas, img, prelude::*, px,
};
use std::sync::{Arc, Mutex};

use super::{Badge, BadgeTone, ButtonVariant, Icon, IconButton, ProgressBar, Slider, Spinner};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type SeekHandler = Arc<dyn Fn(f32, &mut Window, &mut App) + 'static>;
type VolumeHandler = Box<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// The persistent playback bar at the bottom of the window.
#[derive(IntoElement)]
pub struct PlayerBar {
    title: SharedString,
    artist: SharedString,
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
    queue_open: bool,
    on_like: Option<ClickHandler>,
    on_queue: Option<ClickHandler>,
    on_track: Option<ClickHandler>,
    on_mute: Option<ClickHandler>,
    on_volume: Option<VolumeHandler>,
    on_toggle: Option<ClickHandler>,
    on_previous: Option<ClickHandler>,
    on_next: Option<ClickHandler>,
    on_seek: Option<SeekHandler>,
}

impl PlayerBar {
    pub fn new() -> Self {
        Self {
            title: "".into(),
            artist: "".into(),
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
            queue_open: false,
            on_like: None,
            on_queue: None,
            on_track: None,
            on_mute: None,
            on_volume: None,
            on_toggle: None,
            on_previous: None,
            on_next: None,
            on_seek: None,
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

    pub fn on_track(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_track = Some(Box::new(handler));
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

        let mut left_meta = gpui::div()
            .flex()
            .flex_col()
            .min_w(px(0.0))
            .gap(px(2.0))
            .child(
                gpui::div()
                    .text_size(px(text::BASE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.colors.text)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(title),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap(px(space::XS))
                    .text_size(px(text::SM))
                    .text_color(subtitle_color)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(self.error.is_some(), |element| {
                        element.child(
                            Icon::new(icons::WARNING)
                                .size(12.0)
                                .color(theme.colors.danger),
                        )
                    })
                    .child(subtitle),
            );

        if self.loading {
            left_meta = left_meta.child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap(px(space::XS))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child(Spinner::new().size(11.0))
                    .child("Загрузка трека…"),
            );
        }

        let mut left = gpui::div()
            .id("player-track-info")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .w(px(if self.compact { 260.0 } else { 300.0 }))
            .flex_shrink_0()
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

        if has_track && let Some(handler) = self.on_track {
            left = left
                .cursor_pointer()
                .rounded(px(8.0))
                .hover(|style| style.opacity(0.82))
                .on_click(handler);
        }

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
        .variant(ButtonVariant::Primary);

        if !has_track || self.loading {
            toggle = toggle.disabled(true);
        } else if let Some(handler) = self.on_toggle {
            toggle = toggle.on_click(handler);
        }

        let mut previous = IconButton::new("player-previous", icons::PREVIOUS)
            .size(36.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost);
        if !has_track || self.loading {
            previous = previous.disabled(true);
        } else if let Some(handler) = self.on_previous {
            previous = previous.on_click(handler);
        }

        let mut next = IconButton::new("player-next", icons::NEXT)
            .size(36.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost);
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
            .flex()
            .items_center()
            .gap(px(space::SM))
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
            .gap(px(space::SM))
            .child(
                gpui::div()
                    .w(px(38.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .text_right()
                    .child(self.elapsed),
            )
            .child(seek_area(self.progress, self.on_seek))
            .child(
                gpui::div()
                    .w(px(38.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child(self.duration),
            );

        let center = gpui::div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .flex_1()
            .min_w(px(0.0))
            .child(controls)
            .child(progress_row);

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

fn seek_area(progress: f32, on_seek: Option<SeekHandler>) -> gpui::Div {
    let bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> = Arc::new(Mutex::new(None));
    let bounds_for_canvas = bounds.clone();
    let bounds_for_down = bounds.clone();
    let bounds_for_move = bounds;
    let handler = on_seek;
    let handler_for_down = handler.clone();
    let handler_for_move = handler;

    gpui::div()
        .relative()
        .flex_1()
        .h(px(20.0))
        .flex()
        .items_center()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                let Some(handler) = handler_for_down.as_ref() else {
                    return;
                };
                let Ok(bounds) = bounds_for_down.lock() else {
                    return;
                };
                let Some(bounds) = bounds.as_ref() else {
                    return;
                };
                let width = f32::from(bounds.size.width);
                if width <= 0.0 {
                    return;
                }
                let fraction = ((f32::from(event.position.x) - f32::from(bounds.origin.x)) / width)
                    .clamp(0.0, 1.0);
                handler(fraction, window, cx);
            },
        )
        .on_mouse_move(move |event: &gpui::MouseMoveEvent, window, cx| {
            if !event.dragging() {
                return;
            }
            let Some(handler) = handler_for_move.as_ref() else {
                return;
            };
            let Ok(bounds) = bounds_for_move.lock() else {
                return;
            };
            let Some(bounds) = bounds.as_ref() else {
                return;
            };
            let width = f32::from(bounds.size.width);
            if width <= 0.0 {
                return;
            }
            let fraction = ((f32::from(event.position.x) - f32::from(bounds.origin.x)) / width)
                .clamp(0.0, 1.0);
            handler(fraction, window, cx);
        })
        .child(ProgressBar::new(progress).height(4.0))
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
