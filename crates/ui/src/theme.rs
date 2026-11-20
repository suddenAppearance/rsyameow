//! Design tokens and the global theme.

use gpui::{App, Global, Pixels, Rgba, SharedString, px, rgb, rgba};

/// All colors used by the application.
#[derive(Clone, Debug)]
pub struct Colors {
    /// Window background.
    pub background: Rgba,
    /// Sidebar background.
    pub sidebar: Rgba,
    /// Cards and panels.
    pub surface: Rgba,
    /// Slightly raised surfaces.
    pub surface_elevated: Rgba,
    /// Hover state for list rows and ghost buttons.
    pub surface_hover: Rgba,
    /// Pressed / selected state.
    pub surface_active: Rgba,
    /// Subtle borders.
    pub border: Rgba,
    /// Stronger borders and separators.
    pub border_strong: Rgba,
    /// Primary text.
    pub text: Rgba,
    /// Secondary text, descriptions.
    pub text_secondary: Rgba,
    /// Muted hints and metadata.
    pub text_muted: Rgba,
    /// Yandex Music yellow.
    pub accent: Rgba,
    /// Hovered yellow.
    pub accent_hover: Rgba,
    /// Text on a yellow background.
    pub accent_text: Rgba,
    /// Translucent yellow for tinted backgrounds.
    pub accent_soft: Rgba,
    /// Errors.
    pub danger: Rgba,
    /// Translucent red background.
    pub danger_soft: Rgba,
    /// Success / active subscription.
    pub success: Rgba,
    /// Warnings.
    pub warning: Rgba,
    /// Pure white, used for overlays.
    pub white: Rgba,
    /// Modal scrim.
    pub overlay: Rgba,
}

/// Corner radii.
#[derive(Clone, Copy, Debug)]
pub struct Radii {
    pub xs: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
    pub pill: Pixels,
}

/// Font families used by the UI.
#[derive(Clone, Debug)]
pub struct Fonts {
    pub ui: SharedString,
    pub mono: SharedString,
}

/// Global design tokens.
#[derive(Clone, Debug)]
pub struct Theme {
    pub colors: Colors,
    pub radii: Radii,
    pub fonts: Fonts,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// The default dark theme.
    pub fn dark() -> Self {
        Self {
            colors: Colors {
                background: rgb(0x0d0d0f),
                sidebar: rgb(0x0a0a0c),
                surface: rgb(0x151518),
                surface_elevated: rgb(0x1b1b20),
                surface_hover: rgb(0x222228),
                surface_active: rgb(0x2a2a31),
                border: rgb(0x26262c),
                border_strong: rgb(0x37373f),
                text: rgb(0xf5f5f6),
                text_secondary: rgb(0xa6a6b0),
                text_muted: rgb(0x74747f),
                accent: rgb(0xffdb4d),
                accent_hover: rgb(0xffe673),
                accent_text: rgb(0x1a1607),
                accent_soft: rgba(0xffdb4d1f),
                danger: rgb(0xff6b6b),
                danger_soft: rgba(0xff6b6b1a),
                success: rgb(0x5ddb8b),
                warning: rgb(0xffb454),
                white: rgb(0xffffff),
                overlay: rgba(0x00000099),
            },
            radii: Radii {
                xs: px(6.0),
                sm: px(8.0),
                md: px(12.0),
                lg: px(16.0),
                xl: px(22.0),
                pill: px(999.0),
            },
            fonts: Fonts {
                ui: "PT Sans".into(),
                mono: "DejaVu Sans Mono".into(),
            },
        }
    }

    /// Install this theme globally.
    pub fn install(self, cx: &mut App) {
        cx.set_global(self);
    }

    /// Read the current theme.
    pub fn get(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }
}

/// Spacing scale in pixels.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
    pub const HUGE: f32 = 48.0;
}

/// Font sizes in pixels.
pub mod text {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 12.5;
    pub const BASE: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 20.0;
    pub const DISPLAY: f32 = 28.0;
    pub const HERO: f32 = 38.0;
}

/// Asset paths for the bundled icon set. The application is responsible for
/// providing these files through gpui's [`gpui::AssetSource`].
pub mod icons {
    pub const PLAY: &str = "icons/play.svg";
    pub const PAUSE: &str = "icons/pause.svg";
    pub const NEXT: &str = "icons/next.svg";
    pub const PREVIOUS: &str = "icons/previous.svg";
    pub const SHUFFLE: &str = "icons/shuffle.svg";
    pub const LOGOUT: &str = "icons/logout.svg";
    pub const USER: &str = "icons/user.svg";
    pub const HEART: &str = "icons/heart.svg";
    pub const HEART_OUTLINE: &str = "icons/heart-outline.svg";
    pub const SEARCH: &str = "icons/search.svg";
    pub const PLAYLIST: &str = "icons/playlist.svg";
    pub const QUEUE: &str = "icons/queue.svg";
    pub const CLOSE: &str = "icons/close.svg";
    pub const WINDOW_MINIMIZE: &str = "icons/window-minimize.svg";
    pub const WINDOW_MAXIMIZE: &str = "icons/window-maximize.svg";
    pub const WINDOW_RESTORE: &str = "icons/window-restore.svg";
    pub const CHEVRON_LEFT: &str = "icons/chevron-left.svg";
    pub const VOLUME: &str = "icons/volume.svg";
    pub const VOLUME_MUTED: &str = "icons/volume-muted.svg";
    pub const MUSIC: &str = "icons/music.svg";
    pub const EXTERNAL: &str = "icons/external.svg";
    pub const REFRESH: &str = "icons/refresh.svg";
    pub const CHEVRON_RIGHT: &str = "icons/chevron-right.svg";
    pub const SPARKLES: &str = "icons/sparkles.svg";
    pub const COPY: &str = "icons/copy.svg";
    pub const WARNING: &str = "icons/warning.svg";
    pub const CHECK: &str = "icons/check.svg";
    pub const EXPLICIT: &str = "icons/explicit.svg";
    pub const PAW: &str = "icons/paw.svg";
}
