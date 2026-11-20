use crate::theme::{Theme, space, text};
use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, Render, SharedString, Window, prelude::*, px,
};

/// Events emitted by [`TextInput`].
#[derive(Clone, Debug)]
pub enum TextInputEvent {
    /// The content changed.
    Changed(String),
    /// The user pressed Enter.
    Submit(String),
}

/// A minimal single-line text input.
///
/// It supports typing, Backspace, Enter and Ctrl/Cmd+V paste. This is enough for
/// pasting an OAuth token and keeps the widget dependency-free. It does not
/// implement IME composition or mouse text selection yet.
pub struct TextInput {
    focus_handle: FocusHandle,
    value: String,
    placeholder: SharedString,
    monospace: bool,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: String::new(),
            placeholder: "Вставьте токен".into(),
            monospace: true,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    pub fn text(&self) -> &str {
        &self.value
    }

    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }

    pub fn set_text(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.value = value.into();
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let command = keystroke.modifiers.control || keystroke.modifiers.platform;

        if command {
            if keystroke.key == "v"
                && let Some(pasted) = cx.read_from_clipboard().and_then(|item| item.text())
            {
                let cleaned: String = pasted
                    .chars()
                    .filter(|ch| !ch.is_control() && *ch != '\n' && *ch != '\r')
                    .collect();
                self.value.push_str(&cleaned);
                cx.emit(TextInputEvent::Changed(self.value.clone()));
                cx.notify();
            }
            let _ = window;
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.value.pop();
                cx.emit(TextInputEvent::Changed(self.value.clone()));
                cx.notify();
            }
            "enter" | "return" => {
                cx.emit(TextInputEvent::Submit(self.value.trim().to_string()));
            }
            _ => {
                if let Some(character) = keystroke.key_char.as_deref()
                    && !character.is_empty()
                    && !character.chars().any(char::is_control)
                {
                    self.value.push_str(character);
                    cx.emit(TextInputEvent::Changed(self.value.clone()));
                    cx.notify();
                }
            }
        }
    }

    fn on_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
    }
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        let focused = self.focus_handle.is_focused(window);
        let empty = self.value.is_empty();

        let text_color = if empty {
            theme.colors.text_muted
        } else {
            theme.colors.text
        };
        let shown: SharedString = if empty {
            self.placeholder.clone()
        } else {
            self.value.clone().into()
        };

        let mut field = gpui::div()
            .id("rsyameow-text-input")
            .track_focus(&self.focus_handle)
            .flex()
            .items_center()
            .gap(px(2.0))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(theme.radii.sm)
            .bg(theme.colors.surface_elevated)
            .border_1()
            .border_color(if focused {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .text_size(px(text::BASE))
            .text_color(text_color)
            .cursor_text()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(
                gpui::div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .when(self.monospace, |element| {
                        element.font_family(theme.fonts.mono.clone())
                    })
                    .child(shown),
            );

        if focused {
            field = field.child(
                gpui::div()
                    .w(px(1.5))
                    .h(px(17.0))
                    .bg(theme.colors.accent)
                    .flex_shrink_0(),
            );
        }

        field
    }
}
