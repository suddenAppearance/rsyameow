//! Liked-track state and mutation helpers.

use super::*;

impl RootView {
    pub(crate) fn is_current_liked(&self) -> bool {
        self.player
            .track
            .as_ref()
            .is_some_and(|track| self.is_track_liked(track))
    }

    pub(crate) fn is_track_liked(&self, track: &Track) -> bool {
        let full_id = track.full_id();
        self.profile.as_ref().is_some_and(|profile| {
            profile
                .liked
                .iter()
                .any(|short| short.id == track.id || short.track_id() == full_id)
        })
    }

    pub(crate) fn is_liked(&self, track: &Track) -> bool {
        self.is_track_liked(track)
    }

    /// Load the next page of details for the liked collection. The initial
    /// profile request only resolves the first 24 tracks; the rest are fetched
    /// as the user scrolls.
    pub(crate) fn load_more_liked(&mut self, cx: &mut Context<Self>) {
        if self.liked_loading_more || self.liked_reloading || self.liked_exhausted || self.demo {
            return;
        }
        let Some(profile) = self.profile.as_ref() else {
            return;
        };
        let ids: Vec<String> = profile
            .liked
            .iter()
            .filter(|short| {
                !profile
                    .sample
                    .iter()
                    .any(|track| track.id == short.id || track.full_id() == short.track_id())
            })
            .take(24)
            .map(TrackShort::track_id)
            .collect();
        if ids.is_empty() {
            self.liked_exhausted = true;
            return;
        }

        self.liked_loading_more = true;
        let epoch = self.profile_epoch;
        let client = self.client.clone();
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx.background_spawn(async move { client.tracks(ids) }).await;
            this.update(cx, |this, cx| {
                if this.profile_epoch != epoch {
                    return;
                }
                this.liked_loading_more = false;
                match result {
                    Ok(tracks) => {
                        let before = this
                            .profile
                            .as_ref()
                            .map(|profile| profile.sample.len())
                            .unwrap_or(0);
                        if let Some(profile) = this.profile.as_mut() {
                            for track in tracks {
                                if !profile.sample.iter().any(|item| {
                                    item.id == track.id || item.full_id() == track.full_id()
                                }) {
                                    profile.sample.push(track);
                                }
                            }
                        }
                        if this
                            .profile
                            .as_ref()
                            .is_none_or(|profile| profile.sample.len() == before)
                        {
                            this.liked_exhausted = true;
                        }
                        let tracks = this
                            .profile
                            .as_ref()
                            .map(|profile| profile.sample.clone())
                            .unwrap_or_default();
                        this.load_covers(tracks, cx);
                        this.liked_error = None;
                    }
                    Err(error) => {
                        this.liked_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Refresh only the liked collection, keeping the current screen and
    /// playback session intact.
    pub(crate) fn reload_liked(&mut self, cx: &mut Context<Self>) {
        if self.liked_reloading {
            return;
        }
        if self.profile.is_none() {
            return;
        }

        if self.demo {
            let tracks = demo::tracks();
            if let Some(profile) = self.profile.as_mut() {
                profile.liked = demo::liked(&tracks);
                profile.total_liked = 248;
                profile.sample = tracks.clone();
            }
            self.liked_error = None;
            self.load_covers(tracks, cx);
            cx.notify();
            return;
        }

        if self.client.token().is_none() {
            self.liked_error = Some("Нужно войти в аккаунт.".into());
            cx.notify();
            return;
        }

        self.liked_reloading = true;
        self.liked_error = None;
        self.liked_exhausted = false;
        self.liked_reload_epoch = self.liked_reload_epoch.wrapping_add(1);
        let request = self.liked_reload_epoch;
        let epoch = self.profile_epoch;
        let client = self.client.clone();
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
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
                    Ok::<_, MusicError>((liked, sample))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.profile_epoch != epoch || this.liked_reload_epoch != request {
                    return;
                }
                this.liked_reloading = false;
                match result {
                    Ok((liked, sample)) => {
                        let total = liked.tracks.len();
                        if let Some(profile) = this.profile.as_mut() {
                            profile.liked = liked.tracks;
                            profile.total_liked = total;
                            profile.sample = sample.clone();
                        }
                        this.liked_error = None;
                        this.load_covers(sample, cx);
                    }
                    Err(error) => {
                        this.liked_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn toggle_like(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.player.track.clone() else {
            return;
        };
        self.toggle_like_track(track, cx);
    }

    pub(crate) fn toggle_like_track(&mut self, track: Track, cx: &mut Context<Self>) {
        if self.like_busy {
            return;
        }
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

    pub(crate) fn apply_like_state(&mut self, track: Track, liked: bool) {
        let full_id = track.full_id();
        let existing_cover = self.covers.get(&track.id).cloned();
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
}
