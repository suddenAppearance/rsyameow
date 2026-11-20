//! `rsyameow-ui` — a small gpui component kit used by the rsyameow desktop app.
//!
//! The kit is intentionally independent from the Yandex Music client: it only
//! knows about colors, typography, images and callbacks. Components rely on an
//! `AssetSource` that serves the SVG paths listed in [`theme::icons`].

pub mod components;
pub mod theme;

pub use components::{
    Avatar, Badge, BadgeTone, Button, ButtonSize, ButtonVariant, Icon, IconButton, PlayerBar,
    ProgressBar, Spinner, Stat, TextInput, TextInputEvent, TrackRow,
};
pub use theme::{Colors, Fonts, Radii, Theme, icons, space, text};
