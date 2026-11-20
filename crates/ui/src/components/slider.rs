use crate::theme::Theme;
use gpui::{
    App, Bounds, ElementId, Entity, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, RenderOnce, SharedString, Window, canvas, prelude::*, px, relative,
};
use std::sync::{Arc, Mutex};

type ChangeHandler = Arc<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// A compact click-to-set horizontal slider with a `0.0..=1.0` value.
///
/// Used for volume control. The value is changed by clicking anywhere on the
/// track; the slider itself does not own any state.
#[derive(IntoElement)]
pub struct Slider {
    value: f32,
    width: f32,
    id: ElementId,
    on_change: Option<ChangeHandler>,
    on_commit: Option<ChangeHandler>,
}

#[derive(Default)]
struct SliderState {
    dragging: bool,
    last: f32,
}

impl Slider {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: 96.0,
            id: ElementId::Name(SharedString::from("slider")),
            on_change: None,
            on_commit: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    pub fn on_commit(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Arc::new(handler));
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx).clone();
        let value = self.value.clamp(0.0, 1.0);
        let track_bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> = Arc::new(Mutex::new(None));
        let bounds_for_canvas = track_bounds.clone();
        let bounds_for_down = track_bounds.clone();
        let bounds_for_move = track_bounds;
        let handler = self.on_change;
        let handler_for_down = handler.clone();
        let handler_for_move = handler;
        let commit = self.on_commit;
        let commit_for_up = commit.clone();
        let commit_for_up_out = commit;
        let state: Entity<SliderState> = window.use_keyed_state(self.id, cx, |_, _| SliderState {
            dragging: false,
            last: value,
        });
        let state_for_down = state.clone();
        let state_for_move = state.clone();
        let state_for_up = state.clone();
        let state_for_up_out = state;

        gpui::div()
            .relative()
            .w(px(self.width))
            .h(px(18.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                move |event: &MouseDownEvent, window, cx| {
                    let Some(handler) = handler_for_down.as_ref() else {
                        return;
                    };
                    let Ok(bounds) = bounds_for_down.lock() else {
                        return;
                    };
                    let Some(bounds) = bounds.as_ref() else {
                        return;
                    };
                    let width = f32::from(bounds.size.width);
                    if width <= 0.0 {
                        return;
                    }
                    let value = ((f32::from(event.position.x) - f32::from(bounds.origin.x))
                        / width)
                        .clamp(0.0, 1.0);
                    state_for_down.update(cx, |state, _| {
                        state.dragging = true;
                        state.last = value;
                    });
                    handler(value, window, cx);
                },
            )
            .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                if !event.dragging() || !state_for_move.read(cx).dragging {
                    return;
                }
                let Some(handler) = handler_for_move.as_ref() else {
                    return;
                };
                let Ok(bounds) = bounds_for_move.lock() else {
                    return;
                };
                let Some(bounds) = bounds.as_ref() else {
                    return;
                };
                let width = f32::from(bounds.size.width);
                if width <= 0.0 {
                    return;
                }
                let value = ((f32::from(event.position.x) - f32::from(bounds.origin.x)) / width)
                    .clamp(0.0, 1.0);
                state_for_move.update(cx, |state, _| state.last = value);
                handler(value, window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, window, cx| {
                let last = state_for_up.update(cx, |state, _| {
                    if state.dragging {
                        state.dragging = false;
                        Some(state.last)
                    } else {
                        None
                    }
                });
                if let Some(value) = last
                    && let Some(handler) = commit_for_up.as_ref()
                {
                    handler(value, window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, window, cx| {
                let last = state_for_up_out.update(cx, |state, _| {
                    if state.dragging {
                        state.dragging = false;
                        Some(state.last)
                    } else {
                        None
                    }
                });
                if let Some(value) = last
                    && let Some(handler) = commit_for_up_out.as_ref()
                {
                    handler(value, window, cx);
                }
            })
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
