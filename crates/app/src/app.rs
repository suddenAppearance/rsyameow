//! Application state, authentication and playback logic.

use crate::audio::AudioEngine;
use crate::{config, demo, format};
use gpui::{
    ClipboardItem, Context, Entity, Image, Subscription, Task, Timer, WeakEntity, prelude::*,
};
use rand::Rng as _;
use rsyameow_ui::{TextInput, TextInputEvent};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use yandex_music::{Client, DeviceCode, Error as MusicError, Status, Track, TrackShort};

/// Which top-level screen is visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Login,
    Loading,
    Profile,
    LoadError(String),
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
    pub(crate) viewport_width: f32,
    pub(crate) viewport_height: f32,
    window_size_epoch: u64,
    _token_subscription: Subscription,
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

        let ticker = Self::spawn_ticker(cx);
        let viewport = window.viewport_size();

        let mut view = Self {
            screen: Screen::Login,
            client,
            demo: false,
            token_input,
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
            audio: AudioEngine::new(),
            playback_epoch: 0,
            viewport_width: f32::from(viewport.width),
            viewport_height: f32::from(viewport.height),
            window_size_epoch: 0,
            _token_subscription: token_subscription,
            _ticker: ticker,
        };

        if demo_mode {
            view.start_demo(cx);
        } else if token.is_some() {
            view.screen = Screen::Loading;
            view.load_profile(cx);
        }

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

                    if this.demo {
                        this.player.position += Duration::from_millis(500);
                        if let Some(duration) = this.player.track.as_ref().map(Track::duration)
                            && this.player.position >= duration
                        {
                            this.player.position = duration;
                            this.player.playing = false;
                        }
                    } else {
                        this.player.position = this.audio.position();
                        if this.audio.is_finished() {
                            this.player.playing = false;
                            if let Some(duration) = this.player.track.as_ref().map(Track::duration)
                            {
                                this.player.position = duration;
                            }
                        }
                    }

                    cx.notify();
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
        self.player.reset();
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.show_token_input = false;
        self.pending_save_token = None;
        self.token_input.update(cx, |input, cx| input.clear(cx));
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
        self.player.reset();

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

    // ----------------------------------------------------------------- Playback

    pub(crate) fn play_random(&mut self, cx: &mut Context<Self>) {
        if self.player.loading {
            return;
        }

        if self.demo {
            let track = self.profile.as_ref().and_then(|profile| {
                if profile.sample.is_empty() {
                    None
                } else {
                    let index = rand::rng().random_range(0..profile.sample.len());
                    Some(profile.sample[index].clone())
                }
            });
            if let Some(track) = track {
                self.start_demo_track(track, cx);
            } else {
                self.player.error = Some("В демо-коллекции нет треков.".into());
                cx.notify();
            }
            return;
        }

        let Some(profile) = self.profile.as_ref() else {
            self.player.error = Some("Профиль ещё не загружен.".into());
            cx.notify();
            return;
        };

        if profile.liked.is_empty() {
            self.player.error = Some(
                "В «Мне нравится» пока нет треков. Добавьте пару песен в Яндекс Музыке и обновите профиль."
                    .into(),
            );
            cx.notify();
            return;
        }

        let index = rand::rng().random_range(0..profile.liked.len());
        let short = profile.liked[index].clone();
        let known = profile
            .sample
            .iter()
            .find(|track| track.id == short.id || track.full_id() == short.track_id())
            .cloned();

        self.load_and_play(known, short.track_id(), cx);
    }

    pub(crate) fn play_track(&mut self, track: Track, cx: &mut Context<Self>) {
        if self.demo {
            self.start_demo_track(track, cx);
            return;
        }

        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);

        if same_track {
            self.toggle_playback(cx);
            return;
        }

        let id = track.full_id();
        self.load_and_play(Some(track), id, cx);
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

                        this.player.track = Some(track.clone());
                        this.player.position = Duration::ZERO;
                        this.player.quality = Some(format!(
                            "{} · {} кбит/с",
                            downloaded.codec.to_uppercase(),
                            downloaded.bitrate_kbps
                        ));
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

    pub(crate) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
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
