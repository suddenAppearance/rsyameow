use crate::theme::{Theme, icons};
use gpui::{
    Animation, AnimationExt as _, App, IntoElement, RenderOnce, Window, percentage, prelude::*, px,
    svg,
};
use std::time::Duration;

/// A small rotating spinner. Relies on the `icons/refresh.svg` asset.
#[derive(IntoElement)]
pub struct Spinner {
    size: f32,
    color: Option<gpui::Rgba>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: 16.0,
            color: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: impl Into<gpui::Rgba>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(Theme::get(cx).colors.text_secondary);
        svg()
            .path(icons::REFRESH)
            .size(px(self.size))
            .text_color(color)
            .flex_shrink_0()
            .with_animation(
                "rsyameow-spinner",
                Animation::new(Duration::from_millis(900)).repeat(),
                |element, delta| {
                    element.with_transformation(gpui::Transformation::rotate(percentage(delta)))
                },
            )
    }
}
