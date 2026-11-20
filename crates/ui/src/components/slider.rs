use crate::theme::Theme;
use gpui::{
    App, Bounds, IntoElement, MouseButton, MouseDownEvent, RenderOnce, Window, canvas, prelude::*,
    px, relative,
};
use std::sync::{Arc, Mutex};

type ChangeHandler = Box<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// A compact click-to-set horizontal slider with a `0.0..=1.0` value.
///
/// Used for volume control. The value is changed by clicking anywhere on the
/// track; the slider itself does not own any state.
#[derive(IntoElement)]
pub struct Slider {
    value: f32,
    width: f32,
    on_change: Option<ChangeHandler>,
}

impl Slider {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            width: 96.0,
            on_change: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::get(cx);
        let value = self.value.clamp(0.0, 1.0);
        let track_bounds: Arc<Mutex<Option<Bounds<gpui::Pixels>>>> = Arc::new(Mutex::new(None));
        let bounds_for_canvas = track_bounds.clone();
        let bounds_for_click = track_bounds;
        let handler = self.on_change;

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
                    let Some(handler) = handler.as_ref() else {
                        return;
                    };
                    let Ok(bounds) = bounds_for_click.lock() else {
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
                    handler(value, window, cx);
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
