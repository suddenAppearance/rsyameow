use crate::theme::Theme;
use gpui::{
    App, Bounds, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    RenderOnce, Window, canvas, prelude::*, px, relative,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

type SeekHandler = Arc<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// A seek slider that only commits when the mouse button is released.
///
/// While dragging it reports a preview fraction, so the owning view can move
/// the thumb without touching the audio decoder.
#[derive(IntoElement)]
pub struct SeekBar {
    value: f32,
    width: f32,
    on_preview: Option<SeekHandler>,
    on_commit: Option<SeekHandler>,
}

impl SeekBar {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: 420.0,
            on_preview: None,
            on_commit: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn on_preview(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_preview = Some(Arc::new(handler));
        self
    }

    pub fn on_commit(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Arc::new(handler));
        self
    }
}

fn fraction(
    event_x: gpui::Pixels,
    bounds: Option<&Bounds<gpui::Pixels>>,
    width: f32,
) -> Option<f32> {
    let bounds = bounds?;
    if width <= 0.0 {
        return None;
    }
    Some(((f32::from(event_x) - f32::from(bounds.origin.x)) / width).clamp(0.0, 1.0))
}

impl RenderOnce for SeekBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let value = self.value.clamp(0.0, 1.0);
        let width = self.width;
        let track_bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> = Arc::new(Mutex::new(None));
        let bounds_for_canvas = track_bounds.clone();
        let bounds_for_down = track_bounds.clone();
        let bounds_for_move = track_bounds.clone();
        let bounds_for_up = track_bounds.clone();
        let bounds_for_up_out = track_bounds;
        let preview = self.on_preview;
        let commit = self.on_commit;
        let preview_for_down = preview.clone();
        let preview_for_move = preview;
        let commit_for_up = commit.clone();
        let commit_for_up_out = commit;
        let dragging = Rc::new(Cell::new(false));
        let dragging_for_down = dragging.clone();
        let dragging_for_move = dragging.clone();
        let dragging_for_up = dragging.clone();
        let dragging_for_up_out = dragging;

        gpui::div()
            .relative()
            .w(px(self.width))
            .flex_shrink_0()
            .h(px(20.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                move |event: &MouseDownEvent, window, cx| {
                    let Ok(bounds) = bounds_for_down.lock() else {
                        return;
                    };
                    let Some(fraction) = fraction(event.position.x, bounds.as_ref(), width) else {
                        return;
                    };
                    dragging_for_down.set(true);
                    if let Some(handler) = preview_for_down.as_ref() {
                        handler(fraction, window, cx);
                    }
                },
            )
            .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                if !event.dragging() || !dragging_for_move.get() {
                    return;
                }
                let Ok(bounds) = bounds_for_move.lock() else {
                    return;
                };
                let Some(fraction) = fraction(event.position.x, bounds.as_ref(), width) else {
                    return;
                };
                if let Some(handler) = preview_for_move.as_ref() {
                    handler(fraction, window, cx);
                }
            })
            .on_mouse_up(
                MouseButton::Left,
                move |event: &MouseUpEvent, window, cx| {
                    let Ok(bounds) = bounds_for_up.lock() else {
                        return;
                    };
                    let Some(fraction) = fraction(event.position.x, bounds.as_ref(), width) else {
                        return;
                    };
                    if dragging_for_up.replace(false)
                        && let Some(handler) = commit_for_up.as_ref()
                    {
                        handler(fraction, window, cx);
                    }
                },
            )
            .on_mouse_up_out(
                MouseButton::Left,
                move |event: &MouseUpEvent, window, cx| {
                    let Ok(bounds) = bounds_for_up_out.lock() else {
                        return;
                    };
                    let Some(fraction) = fraction(event.position.x, bounds.as_ref(), width) else {
                        return;
                    };
                    if dragging_for_up_out.replace(false)
                        && let Some(handler) = commit_for_up_out.as_ref()
                    {
                        handler(fraction, window, cx);
                    }
                },
            )
            .child(
                gpui::div()
                    .w_full()
                    .h(px(3.0))
                    .rounded_full()
                    .bg(theme.colors.border_strong)
                    .overflow_hidden()
                    .child(
                        gpui::div()
                            .h_full()
                            .w(relative(value))
                            .rounded_full()
                            .bg(theme.colors.accent),
                    ),
            )
            .child(
                canvas(
                    move |bounds, _, _| {
                        if let Ok(mut slot) = bounds_for_canvas.lock() {
                            *slot = Some(bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full(),
            )
    }
}
