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
    cursor: usize,
    selection_anchor: Option<usize>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: String::new(),
            placeholder: "Вставьте токен".into(),
            monospace: true,
            cursor: 0,
            selection_anchor: None,
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
        self.cursor = self.value.chars().count();
        self.selection_anchor = None;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.value.clear();
        self.cursor = 0;
        self.selection_anchor = None;
        cx.notify();
    }

    fn selected_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        let len = self.value.chars().count();
        let anchor = anchor.min(len);
        let cursor = self.cursor.min(len);
        if anchor == cursor {
            None
        } else {
            Some((anchor.min(cursor), anchor.max(cursor)))
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selected_range() else {
            return false;
        };
        let mut chars: Vec<char> = self.value.chars().collect();
        chars.drain(start..end);
        self.value = chars.iter().collect();
        self.cursor = start;
        self.selection_anchor = None;
        true
    }

    fn insert_text(&mut self, text: &str) -> bool {
        let text: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if text.is_empty() {
            return false;
        }
        self.delete_selection();
        let mut chars: Vec<char> = self.value.chars().collect();
        let insert_at = self.cursor.min(chars.len());
        let inserted: Vec<char> = text.chars().collect();
        let inserted_len = inserted.len();
        chars.splice(insert_at..insert_at, inserted);
        self.value = chars.iter().collect();
        self.cursor = insert_at + inserted_len;
        self.selection_anchor = None;
        true
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Changed(self.value.clone()));
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let command = keystroke.modifiers.control || keystroke.modifiers.platform;
        let len = self.value.chars().count();

        if command {
            match keystroke.key.as_str() {
                "a" => {
                    self.selection_anchor = Some(0);
                    self.cursor = len;
                    cx.notify();
                }
                "v" => {
                    if let Some(pasted) = cx.read_from_clipboard().and_then(|item| item.text())
                        && self.insert_text(&pasted)
                    {
                        self.changed(cx);
                    }
                }
                "c" => {
                    if let Some((start, end)) = self.selected_range() {
                        let selected: String =
                            self.value.chars().skip(start).take(end - start).collect();
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                    }
                }
                "x" => {
                    if let Some((start, end)) = self.selected_range() {
                        let selected: String =
                            self.value.chars().skip(start).take(end - start).collect();
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(selected));
                        self.delete_selection();
                        self.changed(cx);
                    }
                }
                _ => {}
            }
            let _ = window;
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                let deleted_selection = self.delete_selection();
                if !deleted_selection && self.cursor > 0 {
                    let mut chars: Vec<char> = self.value.chars().collect();
                    self.cursor = self.cursor.min(chars.len());
                    if self.cursor > 0 {
                        chars.remove(self.cursor - 1);
                        self.cursor -= 1;
                        self.value = chars.iter().collect();
                    }
                }
                if deleted_selection || self.cursor < len {
                    self.changed(cx);
                }
            }
            "delete" => {
                let deleted_selection = self.delete_selection();
                if !deleted_selection {
                    let mut chars: Vec<char> = self.value.chars().collect();
                    self.cursor = self.cursor.min(chars.len());
                    if self.cursor < chars.len() {
                        chars.remove(self.cursor);
                        self.value = chars.iter().collect();
                    }
                }
                if deleted_selection || self.cursor < len {
                    self.changed(cx);
                }
            }
            "left" => {
                self.selection_anchor = None;
                self.cursor = self.cursor.saturating_sub(1);
                cx.notify();
            }
            "right" => {
                self.selection_anchor = None;
                self.cursor = (self.cursor + 1).min(len);
                cx.notify();
            }
            "home" => {
                self.selection_anchor = None;
                self.cursor = 0;
                cx.notify();
            }
            "end" => {
                self.selection_anchor = None;
                self.cursor = len;
                cx.notify();
            }
            "enter" | "return" => {
                cx.emit(TextInputEvent::Submit(self.value.trim().to_string()));
            }
            _ => {
                if let Some(character) = keystroke.key_char.as_deref()
                    && !character.is_empty()
                    && !character.chars().any(char::is_control)
                    && self.insert_text(character)
                {
                    self.changed(cx);
                }
            }
        }
    }

    fn on_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.cursor = self.value.chars().count();
        self.selection_anchor = None;
        cx.notify();
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
        let chars: Vec<char> = self.value.chars().collect();
        let len = chars.len();
        self.cursor = self.cursor.min(len);
        let selection = self.selected_range();

        let text_color = if empty {
            theme.colors.text_muted
        } else {
            theme.colors.text
        };

        let mut content = gpui::div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .when(self.monospace, |element| {
                element.font_family(theme.fonts.mono.clone())
            });

        if empty {
            if focused {
                content = content.child(
                    gpui::div()
                        .w(px(1.5))
                        .h(px(17.0))
                        .bg(theme.colors.accent)
                        .flex_shrink_0(),
                );
            }
            content = content.child(
                gpui::div()
                    .text_color(theme.colors.text_muted)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.placeholder.clone()),
            );
        } else if let Some((start, end)) = selection {
            if start > 0 {
                content = content.child(
                    gpui::div()
                        .flex_shrink()
                        .min_w(px(0.0))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(chars[..start].iter().collect::<String>()),
                );
            }
            content = content.child(
                gpui::div()
                    .flex_shrink_0()
                    .rounded(px(3.0))
                    .bg(gpui::rgba(0xFFDB4D4D))
                    .px(px(1.0))
                    .whitespace_nowrap()
                    .child(chars[start..end].iter().collect::<String>()),
            );
            if end < len {
                content = content.child(
                    gpui::div()
                        .flex_shrink()
                        .min_w(px(0.0))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(chars[end..].iter().collect::<String>()),
                );
            }
            if focused {
                content = content.child(
                    gpui::div()
                        .w(px(1.5))
                        .h(px(17.0))
                        .bg(theme.colors.accent)
                        .flex_shrink_0(),
                );
            }
        } else {
            if self.cursor > 0 {
                content = content.child(
                    gpui::div()
                        .flex_shrink()
                        .min_w(px(0.0))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(chars[..self.cursor].iter().collect::<String>()),
                );
            }
            if focused {
                content = content.child(
                    gpui::div()
                        .w(px(1.5))
                        .h(px(17.0))
                        .bg(theme.colors.accent)
                        .flex_shrink_0(),
                );
            }
            if self.cursor < len {
                content = content.child(
                    gpui::div()
                        .flex_shrink()
                        .min_w(px(0.0))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(chars[self.cursor..].iter().collect::<String>()),
                );
            }
        }

        gpui::div()
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
            .child(content)
    }
}
