//! A tiny off-screen window that keeps the gpui event loop alive while the
//! main window is closed and the app continues in the tray.

use gpui::{
    App, AppContext as _, Bounds, IntoElement, Render, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowHandle, WindowKind, WindowOptions, div, prelude::*, px,
    size,
};
use std::sync::Mutex;

static KEEPER: Mutex<Option<WindowHandle<Keeper>>> = Mutex::new(None);

struct Keeper;

impl Render for Keeper {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// Create the keeper once. It is positioned far off-screen and uses the
/// notification window type on X11, so it is not shown in the taskbar.
pub(crate) fn ensure(cx: &mut App) {
    if KEEPER
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().cloned())
        .is_some_and(|handle| handle.update(cx, |_, _, _| {}).is_ok())
    {
        return;
    }

    let bounds = Bounds {
        origin: gpui::point(px(-10000.0), px(-10000.0)),
        size: size(px(1.0), px(1.0)),
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some("ya-myau".to_string()),
        window_decorations: Some(WindowDecorations::Server),
        ..Default::default()
    };
    if let Ok(handle) = cx.open_window(options, |_, cx| cx.new(|_| Keeper))
        && let Ok(mut slot) = KEEPER.lock()
    {
        *slot = Some(handle);
    }
}
