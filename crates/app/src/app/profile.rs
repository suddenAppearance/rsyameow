//! Account profile loading and refresh/error handling.

use super::*;

impl RootView {
    pub(crate) fn load_profile(&mut self, cx: &mut Context<Self>) {
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
        self.album_like_busy = false;
        self.liked_reloading = false;
        self.liked_loading_more = false;
        self.liked_exhausted = false;
        self.liked_error = None;
        self.liked_reload_epoch = self.liked_reload_epoch.wrapping_add(1);
        self.artist = ArtistState::default();
        self.albums = AlbumsState::default();
        self.album = AlbumState::default();
        self.wave = WaveState::default();
        self.page_track = None;
        self.page_context.clear();
        self.track_lyrics.clear();
        self.preloaded.clear();
        self.prefetching.clear();
        self.lyrics_loading.clear();
        self.track_palettes.clear();
        self.track_backdrops.clear();
        self.backdrop_loading.clear();
        self.queue_source = QueueSource::Liked;
        self.nav_stack.clear();
        self.reset_playlists();
        self.audio.stop();
        cx.notify();

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let status = client.init()?;
                    let liked = client.liked_tracks()?;
                    let liked_album_ids = client.liked_album_ids().unwrap_or_default();
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
                        liked_album_ids: liked_album_ids.into_iter().collect(),
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
                        this.restore_state(cx);
                    }
                    Err(error) => this.handle_profile_error(error, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn handle_profile_error(&mut self, error: MusicError, cx: &mut Context<Self>) {
        match error {
            MusicError::Unauthorized(_) | MusicError::NotAuthenticated => {
                self.pending_save_token = None;
                self.client.clear_token();
                config::clear_token();
                config::clear_app_state();
                self.screen = Screen::Login;
                self.login_error = Some("Яндекс отклонил токен. Войдите в аккаунт заново.".into());
            }
            other => {
                self.screen = Screen::LoadError(error_text(&other));
            }
        }
        cx.notify();
    }

    pub(crate) fn retry_profile(&mut self, cx: &mut Context<Self>) {
        if self.client.token().is_none() {
            self.screen = Screen::Login;
            cx.notify();
            return;
        }
        self.load_profile(cx);
    }
}
