use crate::theme::{Theme, icons, space, text};
use gpui::{
    App, ClickEvent, FontWeight, Image, ImageSource, IntoElement, ObjectFit, RenderOnce,
    SharedString, Window, img, prelude::*, px,
};
use std::sync::Arc;

use super::{Badge, BadgeTone, ButtonVariant, Icon, IconButton, ProgressBar, Spinner};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

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
    on_toggle: Option<ClickHandler>,
    on_next: Option<ClickHandler>,
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
            on_toggle: None,
            on_next: None,
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

        let left = gpui::div()
            .flex()
            .items_center()
            .gap(px(space::MD))
            .w(px(300.0))
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

        let mut next = IconButton::new("player-next", icons::NEXT)
            .size(36.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost);
        if !has_track || self.loading {
            next = next.disabled(true);
        } else if let Some(handler) = self.on_next {
            next = next.on_click(handler);
        }

        let controls = gpui::div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .child(toggle)
            .child(next);

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
            .child(
                gpui::div()
                    .flex_1()
                    .child(ProgressBar::new(self.progress).height(4.0)),
            )
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

        let mut right = gpui::div()
            .w(px(190.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(space::SM));

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
