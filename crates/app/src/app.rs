//! Application state, authentication and playback logic.

use crate::audio::AudioEngine;
use crate::{config, demo, format};
use gpui::{
    ClipboardItem, Context, Entity, FocusHandle, Focusable as _, Image, Subscription, Task, Timer,
    WeakEntity, actions, prelude::*,
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
    Album, Artist, Client, DeviceCode, Error as MusicError, Playlist, Status, Track, TrackShort,
};

/// Which top-level screen is visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Login,
    Loading,
    Profile,
    Search,
    Playlists,
    Playlist,
    LoadError(String),
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

    fn reset(&mut self) {
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
    pub(crate) playlists: PlaylistsState,
    pub(crate) playlist_covers: HashMap<String, Arc<Image>>,
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
    pub(crate) queue_track_cache: HashMap<String, Track>,
    queue_fetching: bool,
    pub(crate) volume: f32,
    previous_volume: f32,
    pub(crate) like_busy: bool,
    pub(crate) viewport_width: f32,
    pub(crate) viewport_height: f32,
    window_size_epoch: u64,
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
        let search_subscription =
            cx.subscribe(&search_input, |this, _input, event: &TextInputEvent, cx| {
                if let TextInputEvent::Submit(_) = event {
                    this.perform_search(cx);
                }
            });

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
            playlists: PlaylistsState::default(),
            playlist_covers: HashMap::new(),
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
            queue_track_cache: HashMap::new(),
            queue_fetching: false,
            volume,
            previous_volume: if volume > 0.0 { volume } else { 0.85 },
            like_busy: false,
            viewport_width: f32::from(viewport.width),
            viewport_height: f32::from(viewport.height),
            window_size_epoch: 0,
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
    fn spawn_ticker(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                Timer::after(Duration::from_millis(500)).await;

                let updated = this.update(cx, |this, cx| {
                    if !this.player.playing || this.player.loading {
                        return;
                    }

                    let mut finished = false;

                    if this.demo {
                        this.player.position += Duration::from_millis(500);
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

    // ------------------------------------------------------------- Authentication

    pub(crate) fn start_device_flow(&mut self, cx: &mut Context<Self>) {
        if self.auth_busy || self.device.is_some() {
            return;
        }

        self.auth_busy = true;
        self.login_error = None;
        self.device = None;
        self.auth_epoch += 1;
        let epoch = self.auth_epoch;
        let client = self.client.clone();
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.request_device_code() })
                .await;

            this.update(cx, |this, cx| {
                if this.auth_epoch != epoch {
                    return;
                }
                this.auth_busy = false;
                match result {
                    Ok(code) => {
                        this.device = Some(DeviceFlow {
                            code,
                            copied: false,
                        });
                        this.spawn_device_polling(cx);
                    }
                    Err(error) => {
                        this.login_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn spawn_device_polling(&mut self, cx: &mut Context<Self>) {
        let Some(flow) = &self.device else {
            return;
        };
        let device_code = flow.code.device_code.clone();
        let interval = flow.code.interval.max(2);
        let expires_in = flow.code.expires_in.max(60);
        let epoch = self.auth_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let deadline = Instant::now() + Duration::from_secs(expires_in);

            loop {
                Timer::after(Duration::from_secs(interval)).await;

                let active = this
                    .update(cx, |this, _| {
                        this.auth_epoch == epoch && this.device.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }

                if Instant::now() >= deadline {
                    this.update(cx, |this, cx| {
                        if this.auth_epoch == epoch {
                            this.device = None;
                            this.login_error =
                                Some("Код подтверждения истёк. Получите новый код.".into());
                            cx.notify();
                        }
                    })
                    .ok();
                    break;
                }

                let client = client.clone();
                let code = device_code.clone();
                let result = cx
                    .background_spawn(async move { client.poll_device_token(&code) })
                    .await;

                match result {
                    Ok(Some(token)) => {
                        let _ = this.update(cx, |this, cx| {
                            if this.auth_epoch == epoch {
                                this.on_authenticated(token.access_token, true, cx);
                            }
                        });
                        break;
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        this.update(cx, |this, cx| {
                            if this.auth_epoch == epoch {
                                this.device = None;
                                this.login_error = Some(error_text(&error));
                                cx.notify();
                            }
                        })
                        .ok();
                        break;
                    }
                }
            }
        })
        .detach();
    }

    pub(crate) fn cancel_device_flow(&mut self, cx: &mut Context<Self>) {
        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        cx.notify();
    }

    pub(crate) fn copy_device_code(&mut self, cx: &mut Context<Self>) {
        let Some(flow) = self.device.as_mut() else {
            return;
        };
        let code = flow.code.user_code.clone();
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        flow.copied = true;
        cx.notify();
    }

    pub(crate) fn begin_login_with_token(
        &mut self,
        token: impl Into<String>,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        let token = token.into().trim().to_string();
        if token.is_empty() {
            self.login_error = Some("Введите OAuth-токен.".into());
            self.show_token_input = true;
            cx.notify();
            return;
        }

        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.demo = false;
        self.pending_save_token = remember.then(|| token.clone());
        self.client.set_token(token.clone());
        self.load_profile(cx);
    }

    fn on_authenticated(&mut self, token: String, remember: bool, cx: &mut Context<Self>) {
        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.pending_save_token = remember.then(|| token.clone());
        self.client.set_token(token.clone());
        self.load_profile(cx);
    }

    pub(crate) fn logout(&mut self, cx: &mut Context<Self>) {
        let was_demo = self.demo;
        self.auth_epoch += 1;
        self.profile_epoch += 1;
        self.playback_epoch += 1;
        self.audio.stop();
        self.client.clear_token();
        if !was_demo {
            // Demo mode does not own the saved session, so leaving it must not
            // delete the user's token.
            config::clear_token();
        }

        self.screen = Screen::Login;
        self.demo = false;
        self.profile = None;
        self.covers.clear();
        self.queue.clear();
        self.queue_index = None;
        self.queue_open = false;
        self.queue_track_cache.clear();
        self.queue_fetching = false;
        self.like_busy = false;
        self.player.reset();
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.show_token_input = false;
        self.pending_save_token = None;
        self.search = SearchState::default();
        self.search_epoch += 1;
        self.reset_playlists();
        self.token_input.update(cx, |input, cx| input.clear(cx));
        self.search_input.update(cx, |input, cx| input.clear(cx));
        cx.notify();
    }

    // ------------------------------------------------------------------ Profile

    fn load_profile(&mut self, cx: &mut Context<Self>) {
        self.profile_epoch += 1;
        let epoch = self.profile_epoch;
        self.screen = Screen::Loading;
        self.login_error = None;
        self.player.reset();
        self.profile = None;
        self.covers.clear();
        self.queue.clear();
        self.queue_index = None;
        self.queue_open = false;
        self.queue_track_cache.clear();
        self.queue_fetching = false;
        self.like_busy = false;
        self.reset_playlists();
        self.audio.stop();
        cx.notify();

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let status = client.init()?;
                    let liked = client.liked_tracks()?;
                    let ids: Vec<String> = liked
                        .tracks
                        .iter()
                        .take(24)
                        .map(TrackShort::track_id)
                        .collect();
                    let sample = if ids.is_empty() {
                        Vec::new()
                    } else {
                        client.tracks(ids)?
                    };
                    Ok::<_, MusicError>(ProfileData {
                        total_liked: liked.tracks.len(),
                        liked: liked.tracks,
                        sample,
                        status,
                    })
                })
                .await;

            this.update(cx, |this, cx| {
                if this.profile_epoch != epoch {
                    return;
                }

                match result {
                    Ok(profile) => {
                        if let Some(token) = this.pending_save_token.take()
                            && let Err(error) = config::save_token(&token)
                        {
                            eprintln!("rsyameow: failed to save session: {error}");
                        }
                        let tracks = profile.sample.clone();
                        this.profile = Some(profile);
                        this.screen = Screen::Profile;
                        this.login_error = None;
                        this.load_covers(tracks, cx);
                    }
                    Err(error) => this.handle_profile_error(error, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn handle_profile_error(&mut self, error: MusicError, cx: &mut Context<Self>) {
        match error {
            MusicError::Unauthorized(_) | MusicError::NotAuthenticated => {
                self.pending_save_token = None;
                self.client.clear_token();
                config::clear_token();
                self.screen = Screen::Login;
                self.login_error = Some("Яндекс отклонил токен. Войдите в аккаунт заново.".into());
            }
            other => {
                self.screen = Screen::LoadError(error_text(&other));
            }
        }
        cx.notify();
    }

    fn load_covers(&mut self, tracks: Vec<Track>, cx: &mut Context<Self>) {
        let epoch = self.profile_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            for track in tracks {
                let already_loaded = this
                    .update(cx, |this, _| this.covers.contains_key(&track.id))
                    .unwrap_or(true);
                if already_loaded {
                    continue;
                }

                let client = client.clone();
                let track_id = track.id.clone();
                let bytes = cx
                    .background_spawn(
                        async move { client.download_cover(&track, 400).ok().flatten() },
                    )
                    .await;

                let Some(bytes) = bytes else {
                    continue;
                };
                let Some(image) = decode_image(bytes) else {
                    continue;
                };

                let still_current = this
                    .update(cx, |this, cx| {
                        if this.profile_epoch != epoch {
                            return false;
                        }
                        this.covers.insert(track_id, image);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !still_current {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn retry_profile(&mut self, cx: &mut Context<Self>) {
        if self.client.token().is_none() {
            self.screen = Screen::Login;
            cx.notify();
            return;
        }
        self.load_profile(cx);
    }

    // -------------------------------------------------------------------- Demo

    pub(crate) fn start_demo(&mut self, cx: &mut Context<Self>) {
        self.auth_epoch += 1;
        self.profile_epoch += 1;
        self.playback_epoch += 1;
        self.audio.stop();
        self.demo = true;
        self.screen = Screen::Profile;
        self.login_error = None;
        self.device = None;
        self.auth_busy = false;
        self.pending_save_token = None;
        self.queue.clear();
        self.queue_index = None;
        self.queue_open = false;
        self.queue_track_cache.clear();
        self.queue_fetching = false;
        self.like_busy = false;
        self.player.reset();
        self.search = SearchState::default();
        self.search_images.clear();
        self.search_epoch += 1;
        self.search_input.update(cx, |input, cx| input.clear(cx));
        self.reset_playlists();

        let tracks = demo::tracks();
        self.covers = demo::covers(&tracks);
        self.profile = Some(ProfileData {
            status: demo::status(),
            liked: demo::liked(&tracks),
            total_liked: 248,
            sample: tracks,
        });
        cx.notify();
    }

    fn start_demo_track(&mut self, track: Track, cx: &mut Context<Self>) {
        self.cache_track(&track);
        self.playback_epoch += 1;
        self.player.track = Some(track.clone());
        self.player.cover = self.covers.get(&track.id).cloned();
        self.player.position = Duration::ZERO;
        self.player.playing = true;
        self.player.loading = false;
        self.player.error = None;
        self.player.quality = Some("MP3 · 320 кбит/с · демо".into());
        cx.notify();
    }

    // ------------------------------------------------------------------- Search

    pub(crate) fn open_search(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Search;
        self.queue_open = false;
        let focus = self.search_input.read(cx).focus_handle(cx);
        window.focus(&focus);
        cx.notify();
    }

    pub(crate) fn open_profile(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Profile;
        cx.notify();
    }

    pub(crate) fn perform_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).text().trim().to_string();
        self.run_search(query, 0, cx);
    }

    pub(crate) fn search_next(&mut self, cx: &mut Context<Self>) {
        if self.search.loading || self.search.query.is_empty() {
            return;
        }
        let next = self.search.page + 1;
        if (next as u64) * self.search.per_page.max(1) >= self.search.total as u64 {
            return;
        }
        let query = self.search.query.clone();
        self.run_search(query, next, cx);
    }

    pub(crate) fn search_previous(&mut self, cx: &mut Context<Self>) {
        if self.search.loading || self.search.page == 0 || self.search.query.is_empty() {
            return;
        }
        let query = self.search.query.clone();
        self.run_search(query, self.search.page - 1, cx);
    }

    fn run_search(&mut self, query: String, page: u32, cx: &mut Context<Self>) {
        if query.is_empty() {
            self.search.searched = true;
            self.search.error = Some("Введите запрос для поиска.".into());
            self.search.loading = false;
            self.search.results.clear();
            self.search.total = 0;
            self.search.page = 0;
            cx.notify();
            return;
        }

        self.search_epoch += 1;
        let epoch = self.search_epoch;
        self.search.query = query.clone();
        self.search.searched = true;
        self.search.loading = true;
        self.search.error = None;
        self.search.page = page;
        self.search.results.clear();
        self.search.total = 0;
        cx.notify();

        if self.demo {
            let needle = query.to_lowercase();
            let total;

            if self.search.kind == SearchKind::Tracks {
                let results: Vec<Track> = self
                    .profile
                    .as_ref()
                    .map(|profile| {
                        profile
                            .sample
                            .iter()
                            .filter(|track| {
                                let haystack = format!(
                                    "{} {} {}",
                                    track.display_title(),
                                    track.artist_names(),
                                    track
                                        .albums
                                        .first()
                                        .and_then(|album| album.title.as_deref())
                                        .unwrap_or_default()
                                )
                                .to_lowercase();
                                haystack.contains(&needle)
                            })
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                total = results.len();
                self.search.results = results.clone();
                self.load_covers(results, cx);
            } else if self.search.kind == SearchKind::Artists {
                let mut seen = HashSet::new();
                let mut artists = Vec::new();
                if let Some(profile) = self.profile.as_ref() {
                    for track in &profile.sample {
                        for artist in &track.artists {
                            if artist
                                .name
                                .as_deref()
                                .is_some_and(|name| name.to_lowercase().contains(&needle))
                                && seen.insert(
                                    artist
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| format!("{:?}", artist.id)),
                                )
                            {
                                artists.push(artist.clone());
                            }
                        }
                    }
                }
                total = artists.len();
                self.search.artists = artists;
            } else {
                let mut seen = HashSet::new();
                let mut albums = Vec::new();
                if let Some(profile) = self.profile.as_ref() {
                    for track in &profile.sample {
                        let artist_names = track.artist_names().to_lowercase();
                        for album in &track.albums {
                            let title = album.title.as_deref().unwrap_or_default();
                            let matches = title.to_lowercase().contains(&needle)
                                || artist_names.contains(&needle);
                            if matches && album.id.is_some_and(|id| seen.insert(id)) {
                                let mut album = album.clone();
                                album.artists = track.artists.clone();
                                albums.push(album);
                            }
                        }
                    }
                }
                total = albums.len();
                self.search.albums = albums;
            }

            self.search.loading = false;
            self.search.total = total;
            self.search.per_page = total.max(1) as u64;
            self.search.page = 0;
            cx.notify();
            return;
        }

        let client = self.client.clone();
        let kind = self.search.kind;
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    match kind {
                        SearchKind::Tracks => client
                            .search_tracks(&query, page)
                            .map(SearchPayload::Tracks),
                        SearchKind::Artists => client
                            .search_artists(&query, page)
                            .map(SearchPayload::Artists),
                        SearchKind::Albums => client
                            .search_albums(&query, page)
                            .map(SearchPayload::Albums),
                    }
                })
                .await;

            this.update(cx, |this, cx| {
                if this.search_epoch != epoch {
                    return;
                }
                this.search.loading = false;
                match result {
                    Ok(payload) => {
                        this.search.error = None;
                        match payload {
                            SearchPayload::Tracks(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.results = found.results;
                                let tracks = this.search.results.clone();
                                this.load_covers(tracks, cx);
                            }
                            SearchPayload::Artists(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.artists = found.results;
                                this.load_search_images(cx);
                            }
                            SearchPayload::Albums(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.albums = found.results;
                                this.load_search_images(cx);
                            }
                        }
                        this.search.page = page;
                    }
                    Err(error) => {
                        this.search.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn set_search_kind(&mut self, kind: SearchKind, cx: &mut Context<Self>) {
        if self.search.kind == kind {
            return;
        }
        self.search.kind = kind;
        let query = self.search_input.read(cx).text().trim().to_string();
        let query = if query.is_empty() {
            self.search.query.clone()
        } else {
            query
        };
        if query.is_empty() {
            self.search.results.clear();
            self.search.artists.clear();
            self.search.albums.clear();
            self.search.total = 0;
            self.search.page = 0;
            self.search.per_page = 0;
            self.search.searched = false;
            self.search.error = None;
            cx.notify();
            return;
        }
        self.run_search(query, 0, cx);
    }

    pub(crate) fn search_for_artist(&mut self, name: String, cx: &mut Context<Self>) {
        self.search.kind = SearchKind::Tracks;
        self.search_input
            .update(cx, |input, cx| input.set_text(name.clone(), cx));
        self.run_search(name, 0, cx);
    }

    pub(crate) fn search_for_album(&mut self, album: Album, cx: &mut Context<Self>) {
        let artists = album.artist_names();
        let query = if artists.is_empty() {
            album.display_title().to_string()
        } else {
            format!("{} {}", artists, album.display_title())
        };
        self.search.kind = SearchKind::Tracks;
        self.search_input
            .update(cx, |input, cx| input.set_text(query.clone(), cx));
        self.run_search(query, 0, cx);
    }

    fn load_search_images(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let epoch = self.search_epoch;
        let mut covers: Vec<(String, String)> = Vec::new();
        for artist in &self.search.artists {
            if let (Some(id), Some(url)) = (artist.id, artist.cover_url(200)) {
                covers.push((format!("artist-{id}"), url));
            }
        }
        for album in &self.search.albums {
            if let (Some(id), Some(url)) = (album.id, album.cover_url(300)) {
                covers.push((format!("album-{id}"), url));
            }
        }
        if covers.is_empty() {
            return;
        }

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            for (key, url) in covers {
                let already = this
                    .update(cx, |this, _| this.search_images.contains_key(&key))
                    .unwrap_or(true);
                if already {
                    continue;
                }
                let client = client.clone();
                let bytes = cx
                    .background_spawn(async move { client.download_image(&url).ok() })
                    .await;
                let Some(bytes) = bytes else {
                    continue;
                };
                let Some(image) = decode_image(bytes) else {
                    continue;
                };
                let keep_going = this
                    .update(cx, |this, cx| {
                        if this.search_epoch != epoch {
                            return false;
                        }
                        this.search_images.insert(key, image);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep_going {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn play_search_result(&mut self, track: Track, cx: &mut Context<Self>) {
        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);

        if same_track {
            self.toggle_playback(cx);
            return;
        }

        let results = self.search.results.clone();
        for item in &results {
            self.cache_track(item);
        }
        self.queue = results.iter().map(Track::full_id).collect();
        self.queue_index = results.iter().position(|item| item.id == track.id);

        if self.demo {
            self.start_demo_track(track, cx);
        } else {
            let full_id = track.full_id();
            self.load_and_play(Some(track), full_id, cx);
        }
    }

    // ------------------------------------------------------------------- Likes

    pub(crate) fn is_current_liked(&self) -> bool {
        self.player
            .track
            .as_ref()
            .is_some_and(|track| self.is_track_liked(track))
    }

    fn is_track_liked(&self, track: &Track) -> bool {
        let full_id = track.full_id();
        self.profile.as_ref().is_some_and(|profile| {
            profile
                .liked
                .iter()
                .any(|short| short.id == track.id || short.track_id() == full_id)
        })
    }

    pub(crate) fn toggle_like(&mut self, cx: &mut Context<Self>) {
        if self.like_busy {
            return;
        }
        let Some(track) = self.player.track.clone() else {
            return;
        };
        let liked = self.is_track_liked(&track);

        if self.demo {
            self.apply_like_state(track, !liked);
            cx.notify();
            return;
        }

        if self.client.token().is_none() {
            self.player.error = Some("Нужно войти в аккаунт.".into());
            cx.notify();
            return;
        }

        self.like_busy = true;
        let client = self.client.clone();
        let track_id = track.id.clone();
        let track_for_ui = track.clone();
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    if liked {
                        client.unlike_tracks(&[track_id])
                    } else {
                        client.like_tracks(&[track_id])
                    }
                })
                .await;

            this.update(cx, |this, cx| {
                this.like_busy = false;
                match result {
                    Ok(()) => this.apply_like_state(track_for_ui, !liked),
                    Err(error) => this.player.error = Some(error_text(&error)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn apply_like_state(&mut self, track: Track, liked: bool) {
        let full_id = track.full_id();
        let existing_cover = self.player.cover.clone();
        let Some(profile) = self.profile.as_mut() else {
            return;
        };

        let already_liked = profile
            .liked
            .iter()
            .any(|short| short.id == track.id || short.track_id() == full_id);

        if liked {
            if !already_liked {
                let album_id = track
                    .albums
                    .first()
                    .and_then(|album| album.id)
                    .map(|id| id.to_string());
                profile.liked.insert(
                    0,
                    TrackShort {
                        id: track.id.clone(),
                        album_id,
                        timestamp: None,
                        play_count: None,
                        recent: None,
                        original_index: None,
                        track: None,
                    },
                );
                profile.total_liked += 1;
            }

            if !profile.sample.iter().any(|item| item.id == track.id) {
                profile.sample.insert(0, track.clone());
                profile.sample.truncate(24);
            }
        } else {
            let before = profile.liked.len();
            profile
                .liked
                .retain(|short| short.id != track.id && short.track_id() != full_id);
            if profile.liked.len() < before {
                profile.total_liked = profile.total_liked.saturating_sub(1);
            }
            profile.sample.retain(|item| item.id != track.id);
        }

        if liked && let Some(cover) = existing_cover {
            self.covers.entry(track.id).or_insert(cover);
        }
    }

    // ---------------------------------------------------------------- Playlists

    fn reset_playlists(&mut self) {
        let epoch = self.playlists.epoch.wrapping_add(1);
        let detail_epoch = self.playlists.detail_epoch.wrapping_add(1);
        self.playlists = PlaylistsState {
            epoch,
            detail_epoch,
            ..PlaylistsState::default()
        };
        self.playlist_covers.clear();
    }

    pub(crate) fn open_playlists(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Playlists;
        if !self.playlists.loaded && !self.playlists.loading {
            self.load_playlists(cx);
        }
        cx.notify();
    }

    pub(crate) fn load_playlists(&mut self, cx: &mut Context<Self>) {
        self.playlists.loading = true;
        self.playlists.error = None;
        self.playlists.epoch = self.playlists.epoch.wrapping_add(1);
        let epoch = self.playlists.epoch;
        self.screen = Screen::Playlists;
        cx.notify();

        if self.demo {
            let tracks = self
                .profile
                .as_ref()
                .map(|profile| profile.sample.clone())
                .unwrap_or_default();
            self.playlists.items = demo::playlists(&tracks);
            self.playlists.loading = false;
            self.playlists.loaded = true;
            cx.notify();
            return;
        }

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx.background_spawn(async move { client.playlists() }).await;

            this.update(cx, |this, cx| {
                if this.playlists.epoch != epoch {
                    return;
                }
                this.playlists.loading = false;
                match result {
                    Ok(items) => {
                        this.playlists.items = items.clone();
                        this.playlists.loaded = true;
                        this.playlists.error = None;
                        this.load_playlist_covers(items, cx);
                    }
                    Err(error) => {
                        this.playlists.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_playlist_covers(&mut self, playlists: Vec<Playlist>, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let epoch = self.playlists.epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            for playlist in playlists {
                let id = playlist.playlist_id();
                if id.is_empty() {
                    continue;
                }
                let already = this
                    .update(cx, |this, _| this.playlist_covers.contains_key(&id))
                    .unwrap_or(true);
                if already {
                    continue;
                }
                let Some(url) = playlist.cover_url(400) else {
                    continue;
                };
                let client = client.clone();
                let bytes = cx
                    .background_spawn(async move { client.download_image(&url).ok() })
                    .await;
                let Some(bytes) = bytes else {
                    continue;
                };
                let Some(image) = decode_image(bytes) else {
                    continue;
                };
                let keep_going = this
                    .update(cx, |this, cx| {
                        if this.playlists.epoch != epoch {
                            return false;
                        }
                        this.playlist_covers.insert(id, image);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep_going {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn back_to_playlists(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Playlists;
        self.playlists.selected = None;
        self.playlists.tracks.clear();
        self.playlists.detail_epoch = self.playlists.detail_epoch.wrapping_add(1);
        cx.notify();
    }

    pub(crate) fn open_playlist(&mut self, playlist: Playlist, cx: &mut Context<Self>) {
        self.screen = Screen::Playlist;
        self.playlists.selected = Some(playlist.clone());
        self.playlists.tracks.clear();
        self.playlists.tracks_loading = true;
        self.playlists.tracks_error = None;
        self.playlists.detail_epoch = self.playlists.detail_epoch.wrapping_add(1);
        let epoch = self.playlists.detail_epoch;
        cx.notify();

        if self.demo {
            let shorts = playlist.tracks.clone();
            let tracks = self
                .profile
                .as_ref()
                .map(|profile| {
                    shorts
                        .iter()
                        .filter_map(|short| {
                            profile
                                .sample
                                .iter()
                                .find(|track| {
                                    track.id == short.id || track.full_id() == short.track_id()
                                })
                                .cloned()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            self.playlists.tracks_loading = false;
            self.playlists.tracks = tracks.clone();
            self.load_covers(tracks, cx);
            cx.notify();
            return;
        }

        let (Some(uid), Some(kind)) = (playlist.uid, playlist.kind) else {
            self.playlists.tracks_loading = false;
            self.playlists.tracks_error = Some("У плейлиста нет uid или kind.".into());
            cx.notify();
            return;
        };

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let detail = client.user_playlist(uid, kind)?;
                    let ids: Vec<String> = detail
                        .tracks
                        .iter()
                        .take(100)
                        .map(TrackShort::track_id)
                        .collect();
                    let tracks = if ids.is_empty() {
                        Vec::new()
                    } else {
                        client.tracks(ids)?
                    };
                    Ok::<_, MusicError>((detail, tracks))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.playlists.detail_epoch != epoch {
                    return;
                }
                this.playlists.tracks_loading = false;
                match result {
                    Ok((detail, tracks)) => {
                        if let Some(item) = this
                            .playlists
                            .items
                            .iter_mut()
                            .find(|item| item.playlist_id() == detail.playlist_id())
                        {
                            *item = detail.clone();
                        }
                        this.playlists.selected = Some(detail);
                        this.playlists.tracks = tracks.clone();
                        this.playlists.tracks_error = None;
                        this.load_covers(tracks, cx);
                    }
                    Err(error) => {
                        this.playlists.tracks_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn play_playlist_track(&mut self, track: Track, cx: &mut Context<Self>) {
        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);
        if same_track {
            self.toggle_playback(cx);
            return;
        }

        let tracks = self.playlists.tracks.clone();
        for item in &tracks {
            self.cache_track(item);
        }
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = tracks.iter().position(|item| item.id == track.id);

        if self.demo {
            self.start_demo_track(track, cx);
        } else {
            let full_id = track.full_id();
            self.load_and_play(Some(track), full_id, cx);
        }
    }

    pub(crate) fn play_playlist(&mut self, cx: &mut Context<Self>) {
        let tracks = self.playlists.tracks.clone();
        if tracks.is_empty() {
            self.player.error = Some("В плейлисте нет треков.".into());
            cx.notify();
            return;
        }
        for item in &tracks {
            self.cache_track(item);
        }
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = None;
        self.play_queue_index(0, cx);
    }

    pub(crate) fn shuffle_playlist(&mut self, cx: &mut Context<Self>) {
        let tracks = self.playlists.tracks.clone();
        if tracks.is_empty() {
            self.player.error = Some("В плейлисте нет треков.".into());
            cx.notify();
            return;
        }
        for item in &tracks {
            self.cache_track(item);
        }
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = None;
        let index = rand::rng().random_range(0..tracks.len());
        self.play_queue_index(index, cx);
    }

    // -------------------------------------------------------------------- Queue

    pub(crate) fn toggle_queue(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.queue_open = !self.queue_open;
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
        cx.notify();
    }

    pub(crate) fn close_queue(&mut self, cx: &mut Context<Self>) {
        self.queue_open = false;
        cx.notify();
    }

    fn cache_track(&mut self, track: &Track) {
        self.queue_track_cache
            .insert(track.id.clone(), track.clone());
        self.queue_track_cache
            .insert(track.full_id(), track.clone());
    }

    /// Slice of the queue shown in the side panel.
    pub(crate) fn queue_window(&self) -> (usize, usize) {
        let len = self.queue.len();
        if len == 0 {
            return (0, 0);
        }
        if len <= 40 {
            return (0, len);
        }
        let center = self.queue_index.unwrap_or(0).min(len - 1);
        let start = center.saturating_sub(2);
        let end = (start + 40).min(len);
        (start, end)
    }

    fn fetch_queue_tracks(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            if let Some(profile) = self.profile.as_ref() {
                let sample = profile.sample.clone();
                for track in &sample {
                    self.cache_track(track);
                }
            }
            return;
        }
        if self.queue_fetching || self.client.token().is_none() {
            return;
        }

        let (start, end) = self.queue_window();
        let missing: Vec<String> = self.queue[start..end]
            .iter()
            .filter(|id| !self.queue_track_cache.contains_key(*id))
            .take(40)
            .cloned()
            .collect();
        if missing.is_empty() {
            return;
        }

        self.queue_fetching = true;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.tracks(missing) })
                .await;
            this.update(cx, |this, cx| {
                this.queue_fetching = false;
                if let Ok(tracks) = result {
                    for track in &tracks {
                        this.cache_track(track);
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn play_queue_at(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.queue.len() {
            return;
        }
        self.play_queue_index(index, cx);
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
    }

    // ----------------------------------------------------------------- Playback

    /// Build the playback queue from the loaded profile on first use.
    fn ensure_queue(&mut self) -> bool {
        if !self.queue.is_empty() {
            return true;
        }

        if let Some(profile) = self.profile.as_ref() {
            self.queue = if self.demo {
                profile
                    .sample
                    .iter()
                    .map(|track| track.id.clone())
                    .collect()
            } else {
                profile.liked.iter().map(TrackShort::track_id).collect()
            };
        }

        !self.queue.is_empty()
    }

    fn queue_empty_error(&mut self, cx: &mut Context<Self>) {
        self.player.error = Some(if self.profile.is_none() {
            "Профиль ещё не загружен.".to_string()
        } else if self.demo {
            "В демо-коллекции нет треков.".to_string()
        } else {
            "В «Мне нравится» пока нет треков. Добавьте пару песен в Яндекс Музыке и обновите профиль.".to_string()
        });
        cx.notify();
    }

    fn play_queue_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(id) = self.queue.get(index).cloned() else {
            self.queue_empty_error(cx);
            return;
        };

        let Some(profile) = self.profile.as_ref() else {
            self.queue_empty_error(cx);
            return;
        };

        self.queue_index = Some(index);

        if self.demo {
            let track = profile
                .sample
                .iter()
                .find(|track| track.id == id || track.full_id() == id)
                .cloned();
            if let Some(track) = &track {
                self.cache_track(track);
            }
            match track {
                Some(track) => self.start_demo_track(track, cx),
                None => {
                    self.player.error = Some("Демо-трек не найден.".into());
                    cx.notify();
                }
            }
            return;
        }

        let known = profile
            .sample
            .iter()
            .find(|track| track.id == id || track.full_id() == id)
            .cloned();
        if let Some(track) = &known {
            self.cache_track(track);
        }
        self.load_and_play(known, id, cx);
    }

    pub(crate) fn play_random(&mut self, cx: &mut Context<Self>) {
        if self.player.loading {
            return;
        }

        if !self.ensure_queue() {
            self.queue_empty_error(cx);
            return;
        }

        let index = rand::rng().random_range(0..self.queue.len());
        self.play_queue_index(index, cx);
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
    }

    pub(crate) fn play_next(&mut self, cx: &mut Context<Self>) {
        if !self.ensure_queue() {
            self.queue_empty_error(cx);
            return;
        }

        let index = step_index(self.queue_index, self.queue.len(), true);
        self.play_queue_index(index, cx);
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
    }

    pub(crate) fn play_previous(&mut self, cx: &mut Context<Self>) {
        if !self.ensure_queue() {
            self.queue_empty_error(cx);
            return;
        }

        let index = step_index(self.queue_index, self.queue.len(), false);
        self.play_queue_index(index, cx);
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
    }

    pub(crate) fn play_track(&mut self, track: Track, cx: &mut Context<Self>) {
        self.cache_track(&track);
        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);

        if same_track {
            self.toggle_playback(cx);
            return;
        }

        self.ensure_queue();
        let full_id = track.full_id();
        self.queue_index = self
            .queue
            .iter()
            .position(|id| id == &full_id || id == &track.id);

        if self.demo {
            self.start_demo_track(track, cx);
            return;
        }

        self.load_and_play(Some(track), full_id, cx);
    }

    fn load_and_play(
        &mut self,
        track: Option<Track>,
        requested_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.client.token().is_none() {
            self.player.error = Some("Нужно войти в аккаунт.".into());
            cx.notify();
            return;
        }

        self.playback_epoch += 1;
        let epoch = self.playback_epoch;
        let client = self.client.clone();
        let known = track.clone();

        if let Some(track) = &track {
            self.player.track = Some(track.clone());
            self.player.cover = self.covers.get(&track.id).cloned();
        }
        self.player.position = Duration::ZERO;
        self.player.playing = false;
        self.player.loading = true;
        self.player.error = None;
        self.player.quality = None;
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let track = match known {
                        Some(track) => track,
                        None => client.track(&requested_id)?,
                    };
                    let downloaded = client.download_track(&track.id)?;
                    let cover = client.download_cover(&track, 600).ok().flatten();
                    Ok::<_, MusicError>((track, downloaded, cover))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.playback_epoch != epoch {
                    return;
                }

                this.player.loading = false;

                match result {
                    Ok((track, downloaded, cover)) => {
                        if let Some(bytes) = cover
                            && let Some(image) = decode_image(bytes)
                        {
                            this.covers.insert(track.id.clone(), image.clone());
                            this.player.cover = Some(image);
                        }

                        this.cache_track(&track);
                        this.player.track = Some(track.clone());
                        this.player.position = Duration::ZERO;
                        this.player.quality = Some(
                            if downloaded.codec.to_ascii_lowercase().starts_with("flac") {
                                "FLAC · Lossless".to_string()
                            } else {
                                format!(
                                    "{} · {} кбит/с",
                                    downloaded.codec.to_uppercase(),
                                    downloaded.bitrate_kbps
                                )
                            },
                        );
                        this.player.error = None;

                        match this.audio.load(downloaded.bytes) {
                            Ok(()) => {
                                this.player.playing = true;
                            }
                            Err(error) => {
                                this.player.playing = false;
                                this.player.error = Some(error);
                            }
                        }
                    }
                    Err(error) => {
                        this.player.playing = false;
                        this.player.error = Some(error_text(&error));
                    }
                }

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        let volume = volume.clamp(0.0, 1.0);
        self.volume = volume;
        if volume > 0.0 {
            self.previous_volume = volume;
        }
        self.audio.set_volume(volume);
        if let Err(error) = config::save_volume(volume) {
            eprintln!("rsyameow: failed to save volume: {error}");
        }
        cx.notify();
    }

    pub(crate) fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        if self.volume > 0.0 {
            self.previous_volume = self.volume;
            self.set_volume(0.0, cx);
        } else {
            self.set_volume(self.previous_volume.max(0.2), cx);
        }
    }

    pub(crate) fn seek_to(&mut self, fraction: f32, cx: &mut Context<Self>) {
        if self.player.loading {
            return;
        }
        let Some(track) = self.player.track.clone() else {
            return;
        };
        let duration = track.duration();
        if duration.is_zero() {
            return;
        }

        let target = duration.mul_f32(fraction.clamp(0.0, 1.0));
        if self.demo {
            self.player.position = target;
            cx.notify();
            return;
        }

        match self.audio.seek(target) {
            Ok(()) => {
                self.player.position = target;
                self.player.error = None;
            }
            Err(error) => {
                self.player.error = Some(error);
            }
        }
        cx.notify();
    }

    pub(crate) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        if self.player.loading {
            return;
        }

        if self.demo {
            let Some(track) = self.player.track.clone() else {
                self.play_random(cx);
                return;
            };

            if self.player.playing {
                self.player.playing = false;
            } else {
                if self.player.position >= track.duration() {
                    self.player.position = Duration::ZERO;
                }
                self.player.playing = true;
            }
            cx.notify();
            return;
        }

        let Some(track) = self.player.track.clone() else {
            self.play_random(cx);
            return;
        };

        if self.player.playing {
            self.audio.pause();
            self.player.playing = false;
            cx.notify();
            return;
        }

        if self.audio.is_finished() {
            let id = track.full_id();
            self.load_and_play(Some(track), id, cx);
            return;
        }

        match self.audio.play() {
            Ok(()) => {
                self.player.playing = true;
                self.player.error = None;
            }
            Err(error) => {
                self.player.playing = false;
                self.player.error = Some(error);
            }
        }
        cx.notify();
    }
}

impl Drop for RootView {
    fn drop(&mut self) {
        // A resize might happen immediately before the window is closed; make
        // sure the latest size is not lost.
        let _ = config::save_window_size(self.viewport_width, self.viewport_height);
    }
}

/// Advance a queue position with wrap-around.
fn step_index(current: Option<usize>, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    match current {
        Some(index) if index < len => {
            if forward {
                (index + 1) % len
            } else {
                (index + len - 1) % len
            }
        }
        _ => 0,
    }
}

/// Decode downloaded cover bytes into a gpui image.
pub(crate) fn decode_image(bytes: Vec<u8>) -> Option<Arc<Image>> {
    let format = match image::guess_format(&bytes).ok()? {
        image::ImageFormat::Png => gpui::ImageFormat::Png,
        image::ImageFormat::Jpeg => gpui::ImageFormat::Jpeg,
        image::ImageFormat::WebP => gpui::ImageFormat::Webp,
        image::ImageFormat::Gif => gpui::ImageFormat::Gif,
        image::ImageFormat::Bmp => gpui::ImageFormat::Bmp,
        image::ImageFormat::Tiff => gpui::ImageFormat::Tiff,
        _ => return None,
    };

    Some(Arc::new(Image::from_bytes(format, bytes)))
}

/// A short Russian explanation for API errors.
pub(crate) fn error_text(error: &MusicError) -> String {
    match error {
        MusicError::Network(_) | MusicError::Timeout => {
            "Нет связи с Яндекс Музыкой. Проверьте интернет и попробуйте ещё раз.".into()
        }
        MusicError::Unauthorized(_) => "Яндекс отклонил токен. Войдите в аккаунт заново.".into(),
        MusicError::NotFound(_) => "Трек или аккаунт не найден.".into(),
        MusicError::NoDownload => {
            "Для этого трека нет доступного MP3. Возможно, он доступен только с подпиской Plus."
                .into()
        }
        MusicError::DeviceAuth(message) => {
            format!("Яндекс не подтвердил вход: {message}")
        }
        MusicError::Api { status, .. } => {
            format!("Яндекс Музыка вернула ошибку (HTTP {status}). Попробуйте позже.")
        }
        MusicError::NotAuthenticated => "Нужно войти в аккаунт.".into(),
        MusicError::BadRequest(_) | MusicError::Decode(_) => {
            "Сервис вернул неожиданный ответ. Попробуйте ещё раз.".into()
        }
        MusicError::UnknownAccount => "Не удалось определить аккаунт.".into(),
    }
}

/// Read the account year from the registration date for the profile stats.
pub(crate) fn registered_year(status: &Status) -> String {
    status
        .account
        .as_ref()
        .and_then(|account| account.registered_at.as_deref())
        .and_then(|date| date.get(..4))
        .unwrap_or("—")
        .to_string()
}

/// Human-readable plan name.
pub(crate) fn plan_name(status: &Status) -> String {
    if status.has_plus() {
        "Yandex Plus".into()
    } else {
        "Без подписки".into()
    }
}

/// Subscription end date for the profile.
pub(crate) fn plus_until(status: &Status) -> String {
    status
        .subscription
        .as_ref()
        .and_then(|subscription| subscription.end.as_deref())
        .map(format::date)
        .or_else(|| {
            status
                .permissions
                .as_ref()
                .and_then(|permissions| permissions.until.as_deref())
                .map(format::date)
        })
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::step_index;

    #[test]
    fn queue_index_wraps_in_both_directions() {
        assert_eq!(step_index(None, 3, true), 0);
        assert_eq!(step_index(Some(0), 3, true), 1);
        assert_eq!(step_index(Some(2), 3, true), 0);
        assert_eq!(step_index(Some(0), 3, false), 2);
        assert_eq!(step_index(Some(2), 3, false), 1);
        assert_eq!(step_index(Some(5), 3, true), 0);
        assert_eq!(step_index(Some(0), 0, true), 0);
    }
}
