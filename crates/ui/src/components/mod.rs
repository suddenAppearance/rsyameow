//! Small building blocks used by the application.

use gpui::{App, ClickEvent, Window};

/// Boxed click callback shared by interactive components.
pub(crate) type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

mod avatar;
mod badge;
mod button;
mod icon;
mod player_bar;
mod progress;
mod seek_bar;
mod slider;
mod spinner;
mod stat;
mod text_input;
mod track_row;

pub use avatar::Avatar;
pub use badge::{Badge, BadgeTone};
pub use button::{Button, ButtonSize, ButtonVariant, IconButton};
pub use icon::Icon;
pub use player_bar::PlayerBar;
pub use progress::ProgressBar;
pub use seek_bar::SeekBar;
pub use slider::Slider;
pub use spinner::Spinner;
pub use stat::Stat;
pub use text_input::{TextInput, TextInputEvent};
pub use track_row::TrackRow;
