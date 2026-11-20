//! Playlists list/detail loading and playback.

use super::*;

impl RootView {
    pub(crate) fn reset_playlists(&mut self) {
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
        self.nav_stack.clear();
        if !self.playlists.loaded && !self.playlists.loading {
            self.load_playlists(cx);
        }
        self.schedule_state_save(cx);
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

    pub(crate) fn load_playlist_covers(
        &mut self,
        playlists: Vec<Playlist>,
        cx: &mut Context<Self>,
    ) {
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
        self.schedule_state_save(cx);
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
        self.schedule_state_save(cx);
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
        self.queue_source = QueueSource::Playlist;
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
        self.queue_source = QueueSource::Playlist;
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
        self.queue_source = QueueSource::Playlist;
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = None;
        let index = rand::rng().random_range(0..tracks.len());
        self.play_queue_index(index, cx);
    }
}
