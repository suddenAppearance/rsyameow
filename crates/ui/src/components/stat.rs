use crate::theme::{Theme, text};
use gpui::{App, FontWeight, IntoElement, RenderOnce, SharedString, Window, prelude::*, px};

/// A number with a caption, used in the profile stats row.
#[derive(IntoElement)]
pub struct Stat {
    value: SharedString,
    label: SharedString,
    accent: bool,
}

impl Stat {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            accent: false,
        }
    }

    pub fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

impl RenderOnce for Stat {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        gpui::div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .min_w(px(88.0))
            .child(
                gpui::div()
                    .text_size(px(text::XL))
                    .font_weight(FontWeight::BOLD)
                    .text_color(if self.accent {
                        theme.colors.accent
                    } else {
                        theme.colors.text
                    })
                    .child(self.value),
            )
            .child(
                gpui::div()
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child(self.label),
            )
    }
}
