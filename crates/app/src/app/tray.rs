//! StatusNotifier tray icon.

use super::*;
use gpui::{Context, Window};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, Tray};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, Copy)]
pub(crate) enum TrayEvent {
    Show,
    Quit,
}

pub(crate) struct AppTray {
    sender: Sender<TrayEvent>,
}

impl Tray for AppTray {
    fn id(&self) -> String {
        "rsyameow".into()
    }

    fn title(&self) -> String {
        "Я.Мяу".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![tray_icon()]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(TrayEvent::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: "Открыть Я.Мяу".into(),
                activate: Box::new(|tray| {
                    let _ = tray.sender.send(TrayEvent::Show);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Выход".into(),
                activate: Box::new(|tray| {
                    let _ = tray.sender.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }),
        ]
    }
}

impl RootView {
    pub(crate) fn close_window(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if self.tray_handle.is_some() {
            window.minimize_window();
        } else {
            window.remove_window();
        }
    }
}

pub(crate) fn spawn_tray() -> (Option<Handle<AppTray>>, Receiver<TrayEvent>) {
    let (sender, receiver) = mpsc::channel();
    let has_session_bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| std::path::Path::new(&dir).join("bus").exists())
            .unwrap_or(false);
    if !has_session_bus {
        return (None, receiver);
    }
    let tray = AppTray { sender };
    let handle = tray.spawn().ok();
    (handle, receiver)
}

fn tray_icon() -> Icon {
    const SIZE: usize = 64;
    let mut data = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = (px - 32.0).abs() - 16.0;
            let dy = (py - 32.0).abs() - 16.0;
            let outside = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - 12.0;
            if outside > 0.0 {
                continue;
            }
            let index = (y * SIZE + x) * 4;
            let paw = paw_pixel(px, py);
            let (r, g, b) = if paw { (23, 23, 23) } else { (255, 219, 77) };
            data[index] = 255;
            data[index + 1] = r;
            data[index + 2] = g;
            data[index + 3] = b;
        }
    }
    Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data,
    }
}

fn paw_pixel(x: f32, y: f32) -> bool {
    let toes: [(f32, f32, f32); 4] = [
        (22.0, 27.0, 5.0),
        (30.0, 22.0, 5.0),
        (38.0, 22.0, 5.0),
        (46.0, 27.0, 5.0),
    ];
    if toes
        .iter()
        .any(|&(cx, cy, radius)| (x - cx).powi(2) + (y - cy).powi(2) <= radius.powi(2))
    {
        return true;
    }
    (x - 34.0).powi(2) / 13.0f32.powi(2) + (y - 41.0).powi(2) / 9.0f32.powi(2) <= 1.0
}
