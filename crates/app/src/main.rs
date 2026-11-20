//! rsyameow — an unofficial Yandex Music desktop client built with gpui.

mod app;
mod assets;
mod audio;
mod config;
mod demo;
mod format;
mod view;

use app::{
    CloseQueue, FocusSearch, NextTrack, PreviousTrack, RootView, ToggleMute, TogglePlayback,
    ToggleQueue,
};
use assets::Assets;
use gpui::{
    App, AppContext as _, Application, Bounds, KeyBinding, SharedString, WindowBounds,
    WindowOptions, px, size,
};
use rsyameow_ui::Theme;

const HELP: &str = "\
rsyameow — неофициальный клиент Яндекс Музыки на gpui

Использование:
  rsyameow [ОПЦИИ]

Опции:
  --token <TOKEN>   OAuth-токен Яндекса (иначе берётся из YANDEX_MUSIC_TOKEN
                    или из сохранённой сессии)
  --demo            открыть интерфейс с демо-данными, без сети
  -h, --help        показать эту справку
  -V, --version     показать версию

Переменные окружения:
  YANDEX_MUSIC_TOKEN  OAuth-токен
  RSYAMEOW_DEMO       если задано, включает демо-режим
";

#[derive(Debug, Default)]
struct Args {
    token: Option<String>,
    demo: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        demo: std::env::var_os("RSYAMEOW_DEMO").is_some(),
        ..Args::default()
    };

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--token" => {
                let token = iter
                    .next()
                    .ok_or_else(|| "--token требует значения".to_string())?;
                args.token = Some(token);
            }
            "--demo" => args.demo = true,
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("rsyameow {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other if other.starts_with("--token=") => {
                args.token = Some(other["--token=".len()..].to_string());
            }
            other => return Err(format!("неизвестный аргумент: {other}\n\n{HELP}")),
        }
    }

    Ok(args)
}

fn main() {
    env_logger::init();

    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    let token = args
        .token
        .clone()
        .filter(|token| !token.trim().is_empty())
        .or_else(|| {
            std::env::var("YANDEX_MUSIC_TOKEN")
                .ok()
                .filter(|token| !token.trim().is_empty())
        })
        .or_else(config::load_token);

    let demo = args.demo;

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            Theme::dark().install(cx);

            cx.bind_keys([
                KeyBinding::new("ctrl-space", TogglePlayback, Some("rsyameow")),
                KeyBinding::new("ctrl-right", NextTrack, Some("rsyameow")),
                KeyBinding::new("ctrl-left", PreviousTrack, Some("rsyameow")),
                KeyBinding::new("ctrl-m", ToggleMute, Some("rsyameow")),
                KeyBinding::new("ctrl-q", ToggleQueue, Some("rsyameow")),
                KeyBinding::new("escape", CloseQueue, Some("rsyameow")),
                KeyBinding::new("ctrl-f", FocusSearch, Some("rsyameow")),
            ]);

            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let display = cx
                .primary_display()
                .map(|display| display.bounds().size)
                .unwrap_or_else(|| size(px(1440.0), px(960.0)));
            let display_width = f32::from(display.width);
            let display_height = f32::from(display.height);
            let max_width = (display_width - 80.0).max(960.0);
            let max_height = (display_height - 80.0).max(640.0);
            let (width, height) = config::load_window_size()
                .map(|(width, height)| {
                    (
                        width.clamp(960.0, max_width),
                        height.clamp(640.0, max_height),
                    )
                })
                .unwrap_or_else(|| (1240.0f32.min(max_width), 900.0f32.min(max_height)));

            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.0), px(640.0))),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(SharedString::from("rsyameow — Яндекс Музыка")),
                    ..Default::default()
                }),
                app_id: Some("rsyameow".to_string()),
                ..Default::default()
            };

            let token_for_window = token.clone();
            cx.open_window(options, move |window, cx| {
                assets::install_fonts(window);
                let token = token_for_window.clone();
                cx.new(|cx| RootView::new(demo, token, window, cx))
            })
            .expect("failed to open the main window");

            cx.activate(true);
        });
}
