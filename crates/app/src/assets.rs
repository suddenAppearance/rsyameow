use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// Embedded icons and fonts used by the application and the UI kit.
pub struct Assets;

macro_rules! asset {
    ($path:literal) => {
        include_bytes!(concat!("../assets/", $path)).as_slice()
    };
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/play.svg" => asset!("icons/play.svg"),
            "icons/pause.svg" => asset!("icons/pause.svg"),
            "icons/next.svg" => asset!("icons/next.svg"),
            "icons/previous.svg" => asset!("icons/previous.svg"),
            "icons/shuffle.svg" => asset!("icons/shuffle.svg"),
            "icons/logout.svg" => asset!("icons/logout.svg"),
            "icons/user.svg" => asset!("icons/user.svg"),
            "icons/heart.svg" => asset!("icons/heart.svg"),
            "icons/heart-outline.svg" => asset!("icons/heart-outline.svg"),
            "icons/search.svg" => asset!("icons/search.svg"),
            "icons/playlist.svg" => asset!("icons/playlist.svg"),
            "icons/queue.svg" => asset!("icons/queue.svg"),
            "icons/close.svg" => asset!("icons/close.svg"),
            "icons/window-minimize.svg" => asset!("icons/window-minimize.svg"),
            "icons/window-maximize.svg" => asset!("icons/window-maximize.svg"),
            "icons/window-restore.svg" => asset!("icons/window-restore.svg"),
            "icons/chevron-left.svg" => asset!("icons/chevron-left.svg"),
            "icons/volume.svg" => asset!("icons/volume.svg"),
            "icons/volume-muted.svg" => asset!("icons/volume-muted.svg"),
            "icons/music.svg" => asset!("icons/music.svg"),
            "icons/external.svg" => asset!("icons/external.svg"),
            "icons/refresh.svg" => asset!("icons/refresh.svg"),
            "icons/chevron-right.svg" => asset!("icons/chevron-right.svg"),
            "icons/sparkles.svg" => asset!("icons/sparkles.svg"),
            "icons/copy.svg" => asset!("icons/copy.svg"),
            "icons/warning.svg" => asset!("icons/warning.svg"),
            "icons/check.svg" => asset!("icons/check.svg"),
            "fonts/PT_Sans-Web-Regular.ttf" => asset!("fonts/PT_Sans-Web-Regular.ttf"),
            "fonts/PT_Sans-Web-Bold.ttf" => asset!("fonts/PT_Sans-Web-Bold.ttf"),
            _ => return Ok(None),
        };

        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}

/// Register the bundled PT Sans faces with the window's text system.
pub fn install_fonts(window: &mut gpui::Window) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(asset!("fonts/PT_Sans-Web-Regular.ttf")),
        Cow::Borrowed(asset!("fonts/PT_Sans-Web-Bold.ttf")),
    ];
    let _ = window.text_system().add_fonts(fonts);
}
