use crate::theme::Theme;
use gpui::{
    App, FontWeight, Image, ImageSource, IntoElement, ObjectFit, RenderOnce, SharedString, Window,
    img, prelude::*, px,
};
use std::sync::Arc;

/// A round avatar with either an image or generated initials.
#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    image: Option<Arc<Image>>,
    size: f32,
    show_ring: bool,
}

impl Avatar {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            image: None,
            size: 40.0,
            show_ring: false,
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn ring(mut self, show_ring: bool) -> Self {
        self.show_ring = show_ring;
        self
    }

    fn initials(&self) -> String {
        let mut initials = String::new();
        for word in self.name.split_whitespace().take(2) {
            if let Some(ch) = word.chars().next() {
                initials.extend(ch.to_uppercase());
            }
        }
        if initials.is_empty() {
            "♪".to_string()
        } else {
            initials
        }
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let size = self.size;

        let inner = match self.image {
            Some(image) => img(ImageSource::Image(image))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => gpui::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.colors.accent_text)
                .font_weight(FontWeight::BOLD)
                .text_size(px(size * 0.36))
                .child(self.initials())
                .into_any_element(),
        };

        let mut avatar = gpui::div()
            .size(px(size))
            .flex_shrink_0()
            .rounded(px(size))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.accent);

        if self.show_ring {
            avatar = avatar.border_2().border_color(theme.colors.background);
        }

        avatar.child(inner)
    }
}
