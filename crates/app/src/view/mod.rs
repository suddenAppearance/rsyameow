//! Screen rendering, routing and the root gpui element.
mod album;
mod albums;
mod artist;
mod auth;
mod liked;
mod playlists;
mod profile;
mod queue;
mod search;
mod settings;
mod shell;
mod track;
mod wave;
mod widgets;

use shell::{resize_handle, title_bar};

use crate::app::{
    CloseQueue, CloseWindow, FocusSearch, NextTrack, PreviousTrack, QueueSource, RootView, Screen,
    SearchKind, ToggleMute, TogglePlayback, ToggleQueue, default_palette, plan_name, plus_until,
    registered_year,
};
use crate::config;
use crate::format;
use gpui::{
    AnyElement, BorderStyle, Bounds, Context, CursorStyle, Div, ElementId, Focusable as _, Hsla,
    Image, ImageSource, IntoElement, MouseButton, ObjectFit, Render, ResizeEdge, Rgba,
    ScrollHandle, SharedString, Stateful, Window, canvas, div, img, linear_color_stop,
    linear_gradient, point, prelude::*, px, quad, relative, rgba, size,
};
use rsyameow_ui::{
    Avatar, Badge, BadgeTone, Button, ButtonSize, ButtonVariant, Icon, IconButton, PlayerBar,
    SeekBar, Slider, Spinner, Stat, Theme, TrackRow, icons, space, text,
};
use std::sync::Arc;
use yandex_music::{Album, Playlist, Status, Track};

impl RootView {
    fn text_input_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        let focused = window.focused(cx);
        let search = self.search_input.read(cx).focus_handle(cx);
        let token = self.token_input.read(cx).focus_handle(cx);
        focused
            .as_ref()
            .is_some_and(|handle| handle == &search || handle == &token)
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        if (width - self.viewport_width).abs() > 1.0 || (height - self.viewport_height).abs() > 1.0
        {
            self.viewport_width = width;
            self.viewport_height = height;
            self.schedule_window_size_save(cx);
        }

        let theme = Theme::get(cx).clone();
        let screen = self.screen.clone();

        let content: AnyElement = match &screen {
            Screen::Login => self.login_view(&theme, cx),
            Screen::Loading => self.loading_view(&theme),
            Screen::LoadError(message) => self.load_error_view(message, &theme, cx),
            Screen::Profile => self.profile_view(&theme, cx),
            Screen::Liked => self.liked_view(&theme, cx),
            Screen::Albums => self.albums_view(&theme, cx),
            Screen::Search => self.search_view(&theme, cx),
            Screen::Playlists => self.playlists_view(&theme, cx),
            Screen::Playlist => self.playlist_detail_view(&theme, cx),
            Screen::Artist => self.artist_view(&theme, cx),
            Screen::ArtistTracks => self.artist_tracks_view(&theme, cx),
            Screen::ArtistAlbums => self.artist_albums_view(&theme, cx),
            Screen::Album => self.album_view(&theme, cx),
            Screen::Track => self.track_view(&theme, cx),
            Screen::NowPlaying => self.now_playing_view(&theme, cx),
            Screen::Wave => self.wave_view(&theme, cx),
            Screen::Settings => self.settings_view(&theme, cx),
        };

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .key_context("rsyameow")
            .on_action(cx.listener(|this, _: &TogglePlayback, _, cx| this.toggle_playback(cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.is_held || event.keystroke.key != "space" {
                    return;
                }
                if this.text_input_focused(window, cx) {
                    return;
                }
                this.toggle_playback(cx);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|this, _: &NextTrack, _, cx| this.play_next(cx)))
            .on_action(cx.listener(|this, _: &PreviousTrack, _, cx| this.play_previous(cx)))
            .on_action(cx.listener(|this, _: &ToggleMute, _, cx| this.toggle_mute(cx)))
            .on_action(cx.listener(|this, _: &ToggleQueue, _, cx| this.toggle_queue(cx)))
            .on_action(cx.listener(|this, _: &CloseQueue, _, cx| this.close_queue(cx)))
            .on_action(
                cx.listener(|this, _: &FocusSearch, window, cx| this.open_search(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| {
                this.close_window(window, cx);
            }))
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            .font_family(theme.fonts.ui.clone())
            .child(title_bar(
                &screen,
                window,
                &theme,
                self.tray_handle.is_some(),
            ))
            .child(div().flex_1().min_h(px(0.0)).child(content))
            .child(
                resize_handle("resize-top", ResizeEdge::Top, CursorStyle::ResizeUpDown)
                    .absolute()
                    .top(px(0.0))
                    .left(px(8.0))
                    .right(px(8.0))
                    .h(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom",
                    ResizeEdge::Bottom,
                    CursorStyle::ResizeUpDown,
                )
                .absolute()
                .bottom(px(0.0))
                .left(px(8.0))
                .right(px(8.0))
                .h(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-left",
                    ResizeEdge::Left,
                    CursorStyle::ResizeLeftRight,
                )
                .absolute()
                .left(px(0.0))
                .top(px(8.0))
                .bottom(px(8.0))
                .w(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-right",
                    ResizeEdge::Right,
                    CursorStyle::ResizeLeftRight,
                )
                .absolute()
                .right(px(0.0))
                .top(px(8.0))
                .bottom(px(8.0))
                .w(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-top-left",
                    ResizeEdge::TopLeft,
                    CursorStyle::ResizeUpLeftDownRight,
                )
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-top-right",
                    ResizeEdge::TopRight,
                    CursorStyle::ResizeUpRightDownLeft,
                )
                .absolute()
                .top(px(0.0))
                .right(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom-left",
                    ResizeEdge::BottomLeft,
                    CursorStyle::ResizeUpRightDownLeft,
                )
                .absolute()
                .bottom(px(0.0))
                .left(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom-right",
                    ResizeEdge::BottomRight,
                    CursorStyle::ResizeUpLeftDownRight,
                )
                .absolute()
                .bottom(px(0.0))
                .right(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
    }
}
