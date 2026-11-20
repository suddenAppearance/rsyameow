use crate::theme::Theme;
use gpui::{App, IntoElement, RenderOnce, Window, prelude::*, px, relative};

/// A horizontal progress bar.
#[derive(IntoElement)]
pub struct ProgressBar {
    value: f32,
    height: f32,
    color: Option<gpui::Rgba>,
    track_color: Option<gpui::Rgba>,
}

impl ProgressBar {
    /// Create a bar with a value in the `0.0..=1.0` range.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            height: 4.0,
            color: None,
            track_color: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn color(mut self, color: impl Into<gpui::Rgba>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn track_color(mut self, color: impl Into<gpui::Rgba>) -> Self {
        self.track_color = Some(color.into());
        self
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let height = self.height.max(2.0);
        let fraction = self.value.clamp(0.0, 1.0);
        let color = self.color.unwrap_or(theme.colors.accent);
        let track_color = self.track_color.unwrap_or(theme.colors.border);

        gpui::div()
            .w_full()
            .h(px(height))
            .rounded(px(height / 2.0))
            .bg(track_color)
            .overflow_hidden()
            .child(
                gpui::div()
                    .h_full()
                    .w(relative(fraction))
                    .rounded(px(height / 2.0))
                    .bg(color),
            )
    }
}
