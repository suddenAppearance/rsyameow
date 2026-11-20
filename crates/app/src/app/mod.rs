//! Application state, root view construction, routing types and background tasks.
mod album;
mod artist;
mod auth;
mod cover_art;
mod demo_mode;
mod likes;
mod navigation;
mod playback;
mod playlists;
mod presentation;
mod profile;
mod queue;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod track;
mod wave;

pub(crate) use cover_art::{cover_palette, decode_image, error_text};
pub(crate) use playback::step_index;
pub(crate) use presentation::{plan_name, plus_until, registered_year};

use crate::audio::AudioEngine;
use crate::{config, demo, format};
use gpui::{
    ClipboardItem, Context, Entity, FocusHandle, Focusable as _, Hsla, Image, Rgba, Subscription,
    Task, Timer, WeakEntity, actions, prelude::*,
};
use rand::Rng as _;
use rsyameow_ui::{TextInput, TextInputEvent};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
actions!(
    rsyameow,
    [
        TogglePlayback,
        NextTrack,
        PreviousTrack,
        ToggleMute,
        ToggleQueue,
        CloseQueue,
        FocusSearch,
        CloseWindow
    ]
);

use yandex_music::{
    Album, Artist, Client, DeviceCode, DownloadedTrack, Error as MusicError, Playlist, Status,
    Track, TrackShort,
};

/// Which top-level screen is visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Login,
    Loading,
    Profile,
    Liked,
    Search,
    Playlists,
    Playlist,
    Artist,
    ArtistTracks,
    ArtistAlbums,
    Album,
    Track,
    NowPlaying,
    Wave,
    LoadError(String),
}

/// State of the artist page.
#[derive(Default)]
pub(crate) struct ArtistState {
    pub(crate) id: Option<u64>,
    pub(crate) artist: Option<Artist>,
    pub(crate) tracks: Vec<Track>,
    pub(crate) albums: Vec<Album>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) epoch: u64,
    pub(crate) all_tracks: Vec<Track>,
    pub(crate) all_tracks_loading: bool,
    pub(crate) all_tracks_error: Option<String>,
    pub(crate) all_albums: Vec<Album>,
    pub(crate) all_albums_loading: bool,
    pub(crate) all_albums_error: Option<String>,
}

/// Tracks fetched for "Моя волна".
#[derive(Default)]
pub(crate) struct WaveState {
    pub(crate) tracks: Vec<Track>,
    pub(crate) loaded: bool,
    pub(crate) loading: bool,
    pub(crate) fetching: bool,
    pub(crate) error: Option<String>,
    pub(crate) epoch: u64,
}

/// Which collection produced the current playback queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum QueueSource {
    #[default]
    Liked,
    Search,
    Playlist,
    Artist,
    Wave,
}

impl QueueSource {
    pub(crate) fn key(self) -> &'static str {
        match self {
            QueueSource::Liked => "liked",
            QueueSource::Search => "search",
            QueueSource::Playlist => "playlist",
            QueueSource::Artist => "artist",
            QueueSource::Wave => "wave",
        }
    }

    pub(crate) fn from_key(key: &str) -> Self {
        match key {
            "search" => QueueSource::Search,
            "playlist" => QueueSource::Playlist,
            "artist" => QueueSource::Artist,
            "wave" => QueueSource::Wave,
            _ => QueueSource::Liked,
        }
    }
}

/// State of the album page.
#[derive(Default)]
pub(crate) struct AlbumState {
    pub(crate) id: Option<u64>,
    pub(crate) album: Option<Album>,
    pub(crate) tracks: Vec<Track>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) epoch: u64,
}

/// State of the playlists browser.
#[derive(Default)]
pub(crate) struct PlaylistsState {
    pub(crate) items: Vec<Playlist>,
    pub(crate) loading: bool,
    pub(crate) loaded: bool,
    pub(crate) error: Option<String>,
    pub(crate) selected: Option<Playlist>,
    pub(crate) tracks: Vec<Track>,
    pub(crate) tracks_loading: bool,
    pub(crate) tracks_error: Option<String>,
    pub(crate) epoch: u64,
    pub(crate) detail_epoch: u64,
}

/// State of the search page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SearchKind {
    #[default]
    Tracks,
    Artists,
    Albums,
}

enum SearchPayload {
    Tracks(yandex_music::SearchResult<Track>),
    Artists(yandex_music::SearchResult<Artist>),
    Albums(yandex_music::SearchResult<Album>),
}

#[derive(Default)]
pub(crate) struct SearchState {
    pub(crate) query: String,
    pub(crate) kind: SearchKind,
    pub(crate) results: Vec<Track>,
    pub(crate) artists: Vec<Artist>,
    pub(crate) albums: Vec<Album>,
    pub(crate) total: usize,
    pub(crate) page: u32,
    pub(crate) per_page: u64,
    pub(crate) loading: bool,
    pub(crate) searched: bool,
    pub(crate) error: Option<String>,
}

/// Data loaded for the profile page.
pub(crate) struct ProfileData {
    pub(crate) status: Status,
    pub(crate) liked: Vec<TrackShort>,
    pub(crate) total_liked: usize,
    pub(crate) sample: Vec<Track>,
}

/// State of an in-flight OAuth device authorization.
pub(crate) struct DeviceFlow {
    pub(crate) code: DeviceCode,
    pub(crate) copied: bool,
}

/// Current player information rendered in the bottom bar.
#[derive(Default)]
pub(crate) struct PlayerState {
    pub(crate) track: Option<Track>,
    pub(crate) cover: Option<Arc<Image>>,
    pub(crate) playing: bool,
    pub(crate) loading: bool,
    pub(crate) position: Duration,
    pub(crate) error: Option<String>,
    pub(crate) quality: Option<String>,
    /// Whether the audio backend currently contains this track. Restored
    /// tracks are shown paused and loaded lazily on the first Play.
    pub(crate) audio_ready: bool,
}

impl PlayerState {
    pub(crate) fn progress(&self) -> f32 {
        let duration = self.track.as_ref().map(Track::duration).unwrap_or_default();
        if duration.is_zero() {
            0.0
        } else {
            (self.position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0) as f32
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The root view of the application.
pub struct RootView {
    pub(crate) screen: Screen,
    pub(crate) client: Client,
    pub(crate) demo: bool,
    pub(crate) token_input: Entity<TextInput>,
    pub(crate) search_input: Entity<TextInput>,
    pub(crate) search: SearchState,
    pub(crate) search_epoch: u64,
    search_debounce_epoch: u64,
    pub(crate) playlists: PlaylistsState,
    pub(crate) playlist_covers: HashMap<String, Arc<Image>>,
    pub(crate) artist: ArtistState,
    pub(crate) album: AlbumState,
    pub(crate) wave: WaveState,
    pub(crate) page_track: Option<Track>,
    pub(crate) seek_preview: Option<f32>,
    pub(crate) seek_hover: Option<f32>,
    pub(crate) seek_controls_hovered: bool,
    pub(crate) lyrics_expanded: bool,
    pub(crate) page_context: Vec<Track>,
    pub(crate) track_lyrics: HashMap<String, Option<String>>,
    preloaded: HashMap<String, DownloadedTrack>,
    prefetching: HashSet<String>,
    lyrics_loading: HashSet<String>,
    pub(crate) track_palettes: HashMap<String, (Rgba, Rgba)>,
    pub(crate) track_backdrops: HashMap<String, Arc<Image>>,
    backdrop_loading: HashSet<String>,
    pub(crate) queue_source: QueueSource,
    pub(crate) animation_phase: u64,
    pub(crate) nav_stack: Vec<Screen>,
    pub(crate) search_images: HashMap<String, Arc<Image>>,
    pub(crate) show_token_input: bool,
    pub(crate) login_error: Option<String>,
    pub(crate) device: Option<DeviceFlow>,
    pub(crate) auth_busy: bool,
    pub(crate) auth_epoch: u64,
    pub(crate) profile: Option<ProfileData>,
    pub(crate) profile_epoch: u64,
    pub(crate) pending_save_token: Option<String>,
    pub(crate) covers: HashMap<String, Arc<Image>>,
    pub(crate) player: PlayerState,
    pub(crate) audio: AudioEngine,
    pub(crate) playback_epoch: u64,
    pub(crate) queue: Vec<String>,
    pub(crate) queue_index: Option<usize>,
    pub(crate) queue_open: bool,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) queue_track_cache: HashMap<String, Track>,
    queue_fetching: bool,
    pub(crate) volume: f32,
    previous_volume: f32,
    pub(crate) like_busy: bool,
    pub(crate) liked_reloading: bool,
    pub(crate) liked_error: Option<String>,
    liked_reload_epoch: u64,
    pub(crate) viewport_width: f32,
    pub(crate) viewport_height: f32,
    window_size_epoch: u64,
    state_save_epoch: u64,
    pub(crate) focus_handle: FocusHandle,
    _token_subscription: Subscription,
    _search_subscription: Subscription,
    _ticker: Task<()>,
}

impl RootView {
    pub fn new(
        demo_mode: bool,
        token: Option<String>,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let client = match &token {
            Some(token) => Client::with_token(token.clone()),
            None => Client::unauthenticated(),
        };

        let token_input =
            cx.new(|cx| TextInput::new(cx).placeholder("Вставьте OAuth-токен Яндекса…"));

        let token_subscription =
            cx.subscribe(&token_input, |this, _input, event: &TextInputEvent, cx| {
                if let TextInputEvent::Submit(text) = event {
                    this.begin_login_with_token(text.clone(), true, cx);
                }
            });

        let search_input = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("Трек, исполнитель или альбом")
                .monospace(false)
        });
        let search_subscription = cx.subscribe(
            &search_input,
            |this, _input, event: &TextInputEvent, cx| match event {
                TextInputEvent::Changed(text) => this.schedule_search(text.clone(), cx),
                TextInputEvent::Submit(_) => this.perform_search(cx),
            },
        );

        let ticker = Self::spawn_ticker(cx);
        let focus_handle = cx.focus_handle();
        let viewport = window.viewport_size();
        let volume = config::load_volume().unwrap_or(0.85);
        let mut audio = AudioEngine::new();
        audio.set_volume(volume);

        let mut view = Self {
            screen: Screen::Login,
            client,
            demo: false,
            token_input,
            search_input,
            search: SearchState::default(),
            search_epoch: 0,
            search_debounce_epoch: 0,
            playlists: PlaylistsState::default(),
            playlist_covers: HashMap::new(),
            artist: ArtistState::default(),
            album: AlbumState::default(),
            wave: WaveState::default(),
            page_track: None,
            seek_preview: None,
            seek_hover: None,
            seek_controls_hovered: false,
            lyrics_expanded: false,
            page_context: Vec::new(),
            track_lyrics: HashMap::new(),
            preloaded: HashMap::new(),
            prefetching: HashSet::new(),
            lyrics_loading: HashSet::new(),
            track_palettes: HashMap::new(),
            track_backdrops: HashMap::new(),
            backdrop_loading: HashSet::new(),
            queue_source: QueueSource::Liked,
            animation_phase: 0,
            nav_stack: Vec::new(),
            search_images: HashMap::new(),
            show_token_input: false,
            login_error: None,
            device: None,
            auth_busy: false,
            auth_epoch: 0,
            profile: None,
            profile_epoch: 0,
            pending_save_token: None,
            covers: HashMap::new(),
            player: PlayerState::default(),
            audio,
            playback_epoch: 0,
            queue: Vec::new(),
            queue_index: None,
            queue_open: false,
            sidebar_collapsed: false,
            queue_track_cache: HashMap::new(),
            queue_fetching: false,
            volume,
            previous_volume: if volume > 0.0 { volume } else { 0.85 },
            like_busy: false,
            liked_reloading: false,
            liked_error: None,
            liked_reload_epoch: 0,
            viewport_width: f32::from(viewport.width),
            viewport_height: f32::from(viewport.height),
            window_size_epoch: 0,
            state_save_epoch: 0,
            focus_handle,
            _token_subscription: token_subscription,
            _search_subscription: search_subscription,
            _ticker: ticker,
        };

        if demo_mode {
            view.start_demo(cx);
        } else if token.is_some() {
            view.screen = Screen::Loading;
            view.load_profile(cx);
        }

        window.focus(&view.focus_handle);
        view
    }

    /// Persist the window size after a short debounce, so dragging the window
    /// edge does not write to disk on every frame.
    pub(crate) fn schedule_window_size_save(&mut self, cx: &mut Context<Self>) {
        self.window_size_epoch += 1;
        let epoch = self.window_size_epoch;

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            Timer::after(Duration::from_millis(700)).await;
            this.update(cx, |this, _| {
                if this.window_size_epoch == epoch {
                    let _ = config::save_window_size(this.viewport_width, this.viewport_height);
                }
            })
            .ok();
        })
        .detach();
    }

    /// A background task that advances the progress bar and detects the end of
    /// a track. Demo playback is simulated; real playback asks rodio.
    pub(crate) fn spawn_ticker(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                Timer::after(Duration::from_millis(200)).await;

                let updated = this.update(cx, |this, cx| {
                    let immersive = matches!(
                        this.screen,
                        Screen::Track | Screen::NowPlaying | Screen::Wave
                    );
                    if !immersive && this.player.playing {
                        this.animation_phase = this.animation_phase.wrapping_add(1);
                    }
                    if !this.player.playing || this.player.loading {
                        return;
                    }

                    let mut finished = false;

                    if this.demo {
                        this.player.position += Duration::from_millis(200);
                        if let Some(duration) = this.player.track.as_ref().map(Track::duration)
                            && this.player.position >= duration
                        {
                            this.player.position = duration;
                            this.player.playing = false;
                            finished = true;
                        }
                    } else {
                        this.player.position = this.audio.position();
                        if this.audio.is_finished() {
                            this.player.playing = false;
                            if let Some(duration) = this.player.track.as_ref().map(Track::duration)
                            {
                                this.player.position = duration;
                            }
                            finished = true;
                        }
                    }

                    if finished && this.queue.len() > 1 {
                        this.play_next(cx);
                    } else {
                        cx.notify();
                    }
                });

                if updated.is_err() {
                    break;
                }
            }
        })
    }
}

impl Drop for RootView {
    fn drop(&mut self) {
        // A resize might happen immediately before the window is closed; make
        // sure the latest size is not lost.
        let _ = config::save_window_size(self.viewport_width, self.viewport_height);
        self.save_state_now();
    }
}
