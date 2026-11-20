use crate::theme::{Theme, space, text};
use gpui::{
    App, ClickEvent, ElementId, FontWeight, IntoElement, RenderOnce, SharedString, Window,
    prelude::*, px,
};

use super::{ClickHandler, Icon, Spinner};

type HoverHandler = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// Visual style of a [`Button`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Yellow brand button for the main action on a screen.
    Primary,
    /// Filled dark button with a border.
    Secondary,
    /// Transparent button that only appears on hover.
    Ghost,
    /// Tinted yellow button for secondary brand actions.
    Soft,
    /// Destructive action.
    Danger,
}

/// Size of a [`Button`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

impl ButtonSize {
    fn height(self) -> f32 {
        match self {
            Self::Small => 30.0,
            Self::Medium => 38.0,
            Self::Large => 46.0,
        }
    }

    fn padding(self) -> f32 {
        match self {
            Self::Small => 10.0,
            Self::Medium => 14.0,
            Self::Large => 20.0,
        }
    }

    fn font_size(self) -> f32 {
        match self {
            Self::Small => text::SM,
            Self::Medium => text::BASE,
            Self::Large => text::LG,
        }
    }
}

/// A text button with an optional leading icon.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    icon: Option<SharedString>,
    trailing_icon: Option<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    full_width: bool,
    grow: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            trailing_icon: None,
            variant: ButtonVariant::Secondary,
            size: ButtonSize::Medium,
            disabled: false,
            loading: false,
            full_width: false,
            grow: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    pub fn trailing_icon(mut self, path: impl Into<SharedString>) -> Self {
        self.trailing_icon = Some(path.into());
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Let the button fill the remaining space in a flex row.
    pub fn grow(mut self) -> Self {
        self.grow = true;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let (background, hover_background, foreground, border) = match self.variant {
            ButtonVariant::Primary => (
                theme.colors.accent,
                theme.colors.accent_hover,
                theme.colors.accent_text,
                None,
            ),
            ButtonVariant::Secondary => (
                theme.colors.surface_elevated,
                theme.colors.surface_hover,
                theme.colors.text,
                Some(theme.colors.border),
            ),
            ButtonVariant::Ghost => (
                gpui::rgba(0x00000000),
                theme.colors.surface_hover,
                theme.colors.text_secondary,
                None,
            ),
            ButtonVariant::Soft => (
                gpui::rgba(0x00000000),
                theme.colors.accent_soft,
                theme.colors.accent,
                Some(theme.colors.accent),
            ),
            ButtonVariant::Danger => (
                theme.colors.danger_soft,
                theme.colors.surface_hover,
                theme.colors.danger,
                None,
            ),
        };

        let size = self.size;
        let disabled = self.disabled || self.loading;
        let mut button = gpui::div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(space::SM))
            .h(px(size.height()))
            .px(px(size.padding()))
            .rounded(theme.radii.sm)
            .bg(background)
            .text_size(px(size.font_size()))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(foreground)
            .whitespace_nowrap()
            .flex_shrink_0();

        if let Some(border) = border {
            button = button.border_1().border_color(border);
        }

        if self.full_width {
            button = button.w_full();
        }

        if self.grow {
            button = button.flex_1();
        }

        if disabled {
            button = button.opacity(0.55);
        } else {
            button = button
                .cursor_pointer()
                .hover(move |style| style.bg(hover_background));
        }

        if let Some(icon) = self.icon {
            button = button.child(
                Icon::new(icon)
                    .size(size.font_size() + 2.0)
                    .color(foreground),
            );
        }

        if self.loading {
            button = button.child(Spinner::new().size(size.font_size()).color(foreground));
        }

        button = button.child(self.label);

        if let Some(icon) = self.trailing_icon {
            button = button.child(
                Icon::new(icon)
                    .size(size.font_size() + 2.0)
                    .color(foreground),
            );
        }

        if !disabled && let Some(handler) = self.on_click {
            button = button.on_click(handler);
        }

        button
    }
}

/// A square icon-only button.
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    icon: SharedString,
    size: f32,
    icon_size: f32,
    variant: ButtonVariant,
    disabled: bool,
    loading: bool,
    on_click: Option<ClickHandler>,
    on_hover: Option<HoverHandler>,
    occlude: bool,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            size: 36.0,
            icon_size: 18.0,
            variant: ButtonVariant::Ghost,
            disabled: false,
            loading: false,
            on_click: None,
            on_hover: None,
            occlude: false,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(handler));
        self
    }

    pub fn occlude(mut self) -> Self {
        self.occlude = true;
        self
    }
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let (background, hover_background, foreground) = match self.variant {
            ButtonVariant::Primary => (
                theme.colors.accent,
                theme.colors.accent_hover,
                theme.colors.accent_text,
            ),
            ButtonVariant::Soft => (
                theme.colors.accent_soft,
                theme.colors.accent_soft,
                theme.colors.accent,
            ),
            ButtonVariant::Danger => (
                theme.colors.danger_soft,
                theme.colors.surface_hover,
                theme.colors.danger,
            ),
            _ => (
                gpui::rgba(0x00000000),
                theme.colors.surface_hover,
                theme.colors.text_secondary,
            ),
        };

        let disabled = self.disabled;
        let content = if self.loading {
            Spinner::new()
                .size(self.icon_size)
                .color(foreground)
                .into_any_element()
        } else {
            Icon::new(self.icon)
                .size(self.icon_size)
                .color(foreground)
                .into_any_element()
        };
        let mut button = gpui::div()
            .id(self.id)
            .size(px(self.size))
            .rounded(px(self.size))
            .flex()
            .items_center()
            .justify_center()
            .bg(background)
            .flex_shrink_0()
            .child(content);

        if let Some(handler) = self.on_hover {
            button = button.on_hover(move |hovered, window, cx| {
                handler(hovered, window, cx);
            });
        }
        if self.occlude {
            button = button.occlude();
        }

        if disabled {
            button = button.opacity(if self.loading { 0.85 } else { 0.4 });
        } else {
            button = button
                .cursor_pointer()
                .hover(move |style| style.bg(hover_background));
            if let Some(handler) = self.on_click {
                button = button.on_click(handler);
            }
        }

        button
    }
}
