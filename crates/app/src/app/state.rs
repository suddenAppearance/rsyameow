//! Persistence and restoration of the session state.

use super::*;

impl RootView {
    pub(crate) fn screen_key(&self) -> &'static str {
        match self.screen {
            Screen::Profile | Screen::Login | Screen::Loading | Screen::LoadError(_) => "profile",
            Screen::Liked => "liked",
            Screen::Search => "search",
            Screen::Playlists => "playlists",
            Screen::Playlist => "playlist",
            Screen::Artist | Screen::ArtistTracks | Screen::ArtistAlbums => "artist",
            Screen::Album => "album",
            Screen::Track => "track",
            Screen::NowPlaying => "now_playing",
            Screen::Wave => "wave",
        }
    }

    pub(crate) fn search_kind_key(&self) -> &'static str {
        match self.search.kind {
            SearchKind::Tracks => "tracks",
            SearchKind::Artists => "artists",
            SearchKind::Albums => "albums",
        }
    }

    pub(crate) fn capture_state(&self) -> config::AppState {
        config::AppState {
            version: 1,
            demo: self.demo,
            uid: self
                .profile
                .as_ref()
                .and_then(|profile| profile.status.account.as_ref())
                .and_then(|account| account.uid),
            screen: self.screen_key().to_string(),
            search_query: self.search.query.clone(),
            search_kind: self.search_kind_key().to_string(),
            queue: self.queue.clone(),
            queue_index: self.queue_index,
            track_id: self.player.track.as_ref().map(|track| track.id.clone()),
            page_track_id: if matches!(self.screen, Screen::Track) {
                self.page_track
                    .as_ref()
                    .map(|track| track.id.clone())
                    .or_else(|| self.player.track.as_ref().map(|track| track.id.clone()))
            } else {
                None
            },
            position_ms: self.player.position.as_millis().min(u64::MAX as u128) as u64,
            quality: self.player.quality.clone(),
            queue_open: self.queue_open,
            queue_source: self.queue_source.key().to_string(),
            playlist_id: self
                .playlists
                .selected
                .as_ref()
                .map(|playlist| playlist.playlist_id())
                .filter(|id| !id.is_empty()),
            artist_id: if matches!(
                self.screen,
                Screen::Artist | Screen::ArtistTracks | Screen::ArtistAlbums
            ) {
                self.artist.id
            } else {
                None
            },
            album_id: if matches!(self.screen, Screen::Album) {
                self.album.id
            } else {
                None
            },
        }
    }

    pub(crate) fn save_state_now(&self) {
        if self.profile.is_none() {
            return;
        }
        if let Err(error) = config::save_app_state(&self.capture_state()) {
            eprintln!("rsyameow: failed to save state: {error}");
        }
    }

    pub(crate) fn schedule_state_save(&mut self, cx: &mut Context<Self>) {
        if self.profile.is_none() {
            return;
        }
        self.state_save_epoch += 1;
        let epoch = self.state_save_epoch;

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            Timer::after(Duration::from_millis(600)).await;
            this.update(cx, |this, _| {
                if this.state_save_epoch == epoch {
                    this.save_state_now();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn restore_state(&mut self, cx: &mut Context<Self>) {
        let Some(state) = config::load_app_state() else {
            return;
        };
        if state.demo != self.demo {
            return;
        }
        if !self.demo {
            let current_uid = self
                .profile
                .as_ref()
                .and_then(|profile| profile.status.account.as_ref())
                .and_then(|account| account.uid);
            if state.uid.is_some() && state.uid != current_uid {
                return;
            }
        }

        self.search.kind = match state.search_kind.as_str() {
            "artists" => SearchKind::Artists,
            "albums" => SearchKind::Albums,
            _ => SearchKind::Tracks,
        };
        self.search.query = state.search_query.clone();
        if !state.search_query.is_empty() {
            self.search_input.update(cx, |input, cx| {
                input.set_text(state.search_query.clone(), cx)
            });
        }

        if !state.queue.is_empty() {
            self.queue = state.queue.clone();
            self.queue_index = state.queue_index.filter(|index| *index < self.queue.len());
        }
        self.queue_source = QueueSource::from_key(&state.queue_source);

        self.screen = match state.screen.as_str() {
            "liked" => Screen::Liked,
            "search" => Screen::Search,
            "playlists" => Screen::Playlists,
            "playlist" => Screen::Playlist,
            "artist" => Screen::Artist,
            "album" => Screen::Album,
            "track" => Screen::Track,
            "now_playing" => Screen::NowPlaying,
            "wave" => Screen::Wave,
            _ => Screen::Profile,
        };
        if self.screen == Screen::Track && state.track_id.is_none() && state.page_track_id.is_none()
        {
            self.screen = Screen::Profile;
        }

        self.queue_open = state.queue_open && !self.queue.is_empty();

        let restored_position = Duration::from_millis(state.position_ms);
        let track_id = state.track_id.clone();
        let known = track_id.as_ref().and_then(|id| {
            self.profile
                .as_ref()
                .and_then(|profile| profile.sample.iter().find(|track| &track.id == id).cloned())
        });

        if let Some(track) = known {
            self.apply_restored_track(track, restored_position, state.quality.clone(), cx);
        } else if let (Some(track_id), false) = (track_id, self.demo) {
            let client = self.client.clone();
            let epoch = self.profile_epoch;
            let quality = state.quality.clone();
            cx.spawn(async move |this: WeakEntity<Self>, cx| {
                let result = cx
                    .background_spawn(async move { client.track(&track_id) })
                    .await;
                this.update(cx, |this, cx| {
                    if this.profile_epoch != epoch {
                        return;
                    }
                    if let Ok(track) = result {
                        if !this.queue_track_cache.contains_key(&track.id) {
                            this.cache_track(&track);
                        }
                        let existing = this.covers.get(&track.id).cloned();
                        this.player.track = Some(track.clone());
                        this.player.cover = existing;
                        this.player.position = restored_position.min(track.duration());
                        this.player.playing = false;
                        this.player.loading = false;
                        this.player.audio_ready = false;
                        this.player.quality = quality;
                        this.player.error = None;
                        this.load_covers(vec![track], cx);
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }

        if self.screen == Screen::Track {
            let page_id = state
                .page_track_id
                .clone()
                .or_else(|| state.track_id.clone());
            if let Some(page_id) = page_id {
                if let Some(current) = self
                    .player
                    .track
                    .clone()
                    .filter(|track| track.id == page_id)
                {
                    self.page_track = Some(current);
                } else if let Some(known) = self.profile.as_ref().and_then(|profile| {
                    profile
                        .sample
                        .iter()
                        .find(|track| track.id == page_id)
                        .cloned()
                }) {
                    self.page_track = Some(known);
                } else if let Some(cached) = self.queue_track_cache.get(&page_id).cloned() {
                    self.page_track = Some(cached);
                } else if !self.demo && self.client.token().is_some() {
                    let client = self.client.clone();
                    let epoch = self.profile_epoch;
                    cx.spawn(async move |this: WeakEntity<Self>, cx| {
                        let result = cx
                            .background_spawn(async move { client.track(&page_id) })
                            .await;
                        this.update(cx, |this, cx| {
                            if this.profile_epoch == epoch
                                && let Ok(track) = result
                            {
                                this.load_covers(vec![track.clone()], cx);
                                this.load_track_lyrics(track.clone(), cx);
                                this.load_track_backdrop(track.clone(), cx);
                                this.page_track = Some(track);
                                cx.notify();
                            }
                        })
                        .ok();
                    })
                    .detach();
                }
            }
            if self.page_track.is_none() {
                self.page_track = self.player.track.clone();
            }
            if let Some(track) = self.page_track.clone() {
                self.load_covers(vec![track.clone()], cx);
                self.load_track_lyrics(track.clone(), cx);
                self.load_track_backdrop(track, cx);
            }
        }

        if self.screen == Screen::Playlist {
            let parsed = state.playlist_id.as_deref().and_then(|id| {
                let (uid, kind) = id.split_once(':')?;
                Some((uid.parse::<u64>().ok()?, kind.parse::<u64>().ok()?))
            });
            if let Some((uid, kind)) = parsed {
                self.nav_stack.clear();
                self.nav_stack.push(Screen::Playlists);
                self.open_playlist(
                    Playlist {
                        uid: Some(uid),
                        kind: Some(kind),
                        ..Playlist::default()
                    },
                    cx,
                );
            } else {
                self.screen = Screen::Playlists;
                self.load_playlists(cx);
            }
        } else if self.screen == Screen::Artist {
            if let Some(id) = state.artist_id {
                self.artist = ArtistState {
                    id: Some(id),
                    loading: true,
                    epoch: self.artist.epoch.wrapping_add(1),
                    ..ArtistState::default()
                };
                self.load_artist_data(id, None, cx);
            } else {
                self.screen = Screen::Profile;
            }
        } else if self.screen == Screen::Album {
            if let Some(id) = state.album_id {
                self.album = AlbumState {
                    id: Some(id),
                    loading: true,
                    epoch: self.album.epoch.wrapping_add(1),
                    ..AlbumState::default()
                };
                self.load_album_data(id, None, cx);
            } else {
                self.screen = Screen::Profile;
            }
        } else if self.screen == Screen::Wave {
            self.load_wave(false, false, cx);
        } else if self.screen == Screen::Search && !state.search_query.is_empty() {
            self.run_search(state.search_query.clone(), 0, cx);
        } else if self.screen == Screen::Playlists {
            self.load_playlists(cx);
        }

        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
        cx.notify();
    }

    pub(crate) fn apply_restored_track(
        &mut self,
        track: Track,
        position: Duration,
        quality: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.cache_track(&track);
        self.seek_preview = None;
        self.lyrics_expanded = false;
        self.player.track = Some(track.clone());
        self.player.cover = self.covers.get(&track.id).cloned();
        self.player.position = position.min(track.duration());
        self.player.playing = false;
        self.player.loading = false;
        self.player.audio_ready = false;
        self.player.quality = quality;
        self.player.error = None;
        self.load_covers(vec![track.clone()], cx);
        if matches!(
            self.screen,
            Screen::Track | Screen::NowPlaying | Screen::Wave
        ) {
            self.load_track_backdrop(track, cx);
        }
        cx.notify();
    }
}
