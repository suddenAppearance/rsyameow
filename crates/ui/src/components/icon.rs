use crate::theme::Theme;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window, prelude::*, px, svg};

/// A monochrome icon from the bundled SVG set.
#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
    size: f32,
    color: Option<gpui::Rgba>,
}

impl Icon {
    /// Create an icon from an asset path such as [`crate::theme::icons::PLAY`].
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            size: 18.0,
            color: None,
        }
    }

    /// Set the icon size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Override the tint color.
    pub fn color(mut self, color: impl Into<gpui::Rgba>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(Theme::get(cx).colors.text_secondary);
        svg()
            .path(self.path)
            .size(px(self.size))
            .text_color(color)
            .flex_shrink_0()
    }
}
