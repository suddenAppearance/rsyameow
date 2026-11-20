//! Selected and currently-playing track pages, lyrics and navigation.

use super::*;

impl RootView {
    pub(crate) fn load_track_lyrics(&mut self, track: Track, cx: &mut Context<Self>) {
        let id = track.id.clone();
        if self.track_lyrics.contains_key(&id) || self.lyrics_loading.contains(&id) {
            return;
        }
        if self.demo || self.client.token().is_none() {
            return;
        }

        self.lyrics_loading.insert(id.clone());
        let epoch = self.profile_epoch;
        let client = self.client.clone();
        let request_id = id.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.track_lyrics(&request_id) })
                .await;
            this.update(cx, |this, cx| {
                this.lyrics_loading.remove(&id);
                if this.profile_epoch != epoch {
                    return;
                }
                let lyrics = result.ok().flatten().filter(|text| !text.trim().is_empty());
                this.track_lyrics.insert(id, lyrics);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ------------------------------------------------------------- Preloading

    pub(crate) fn toggle_lyrics(&mut self, cx: &mut Context<Self>) {
        self.lyrics_expanded = !self.lyrics_expanded;
        cx.notify();
    }

    pub(crate) fn open_track(&mut self, track: Track, context: Vec<Track>, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.push_navigation();
        self.load_covers(vec![track.clone()], cx);
        self.page_context = context;
        self.page_track = Some(track.clone());
        self.lyrics_expanded = false;
        self.screen = Screen::Track;
        self.queue_open = false;
        self.load_track_lyrics(track.clone(), cx);
        self.load_track_backdrop(track, cx);
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn open_track_from_queue(&mut self, track: Track, cx: &mut Context<Self>) {
        let context = self
            .queue
            .iter()
            .filter_map(|id| self.queue_track_cache.get(id).cloned())
            .collect::<Vec<_>>();
        self.open_track(track, context, cx);
    }

    pub(crate) fn open_track_from_screen(&mut self, track: Track, cx: &mut Context<Self>) {
        let context = match self.screen {
            Screen::Liked => self
                .profile
                .as_ref()
                .map(|profile| profile.sample.clone())
                .unwrap_or_default(),
            Screen::Search => self.search.results.clone(),
            Screen::Playlist => self.playlists.tracks.clone(),
            Screen::Artist | Screen::ArtistTracks => {
                if self.artist.all_tracks.is_empty() {
                    self.artist.tracks.clone()
                } else {
                    self.artist.all_tracks.clone()
                }
            }
            Screen::Album => self.album.tracks.clone(),
            Screen::Wave => self.wave.tracks.clone(),
            _ => vec![track.clone()],
        };
        self.open_track(track, context, cx);
    }

    pub(crate) fn play_page_track(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self
            .page_track
            .clone()
            .or_else(|| self.player.track.clone())
        else {
            return;
        };
        let is_current = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);
        if is_current {
            self.toggle_playback(cx);
            return;
        }
        if !self.page_context.is_empty() {
            let context = self.page_context.clone();
            self.play_context_track(track, context, cx);
        } else {
            self.cache_track(&track);
            self.queue_source = QueueSource::Liked;
            self.queue = vec![track.full_id()];
            self.queue_index = Some(0);
            if self.demo {
                self.start_demo_track(track, cx);
            } else {
                let id = track.full_id();
                self.load_and_play(Some(track), id, cx);
            }
        }
    }
}
