use crate::theme::{Theme, space, text};
use gpui::{App, FontWeight, IntoElement, RenderOnce, SharedString, Window, prelude::*, px};

/// Visual tone of a [`Badge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeTone {
    /// Brand yellow, for Plus and highlights.
    Accent,
    /// Neutral filled badge.
    Neutral,
    /// Success, for an active subscription.
    Success,
    /// Error / warning.
    Danger,
    /// Transparent with a border.
    Outline,
}

/// A compact pill with a label and an optional icon.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: BadgeTone,
    icon: Option<SharedString>,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            tone: BadgeTone::Neutral,
            icon: None,
        }
    }

    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let (background, foreground, border) = match self.tone {
            BadgeTone::Accent => (theme.colors.accent, theme.colors.accent_text, None),
            BadgeTone::Neutral => (
                theme.colors.surface_active,
                theme.colors.text_secondary,
                None,
            ),
            BadgeTone::Success => (theme.colors.accent, theme.colors.accent_text, None),
            BadgeTone::Danger => (
                theme.colors.danger_soft,
                theme.colors.danger,
                Some(theme.colors.danger),
            ),
            BadgeTone::Outline => (
                gpui::rgba(0x00000000),
                theme.colors.text_secondary,
                Some(theme.colors.border_strong),
            ),
        };

        let mut badge = gpui::div()
            .flex()
            .items_center()
            .gap(px(space::XS))
            .h(px(22.0))
            .px(px(space::SM + 2.0))
            .rounded(px(999.0))
            .bg(background)
            .text_size(px(text::XS))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(foreground)
            .whitespace_nowrap()
            .flex_shrink_0();

        if let Some(border) = border {
            badge = badge.border_1().border_color(border);
        }

        if let Some(icon) = self.icon {
            badge = badge.child(super::Icon::new(icon).size(12.0).color(foreground));
        }

        badge.child(self.label)
    }
}
