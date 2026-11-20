//! Local demo library and simulated playback.

use super::*;

impl RootView {
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
        self.liked_reloading = false;
        self.liked_error = None;
        self.liked_reload_epoch = self.liked_reload_epoch.wrapping_add(1);
        self.player.reset();
        self.search = SearchState::default();
        self.search_images.clear();
        self.search_epoch += 1;
        self.search_input.update(cx, |input, cx| input.clear(cx));
        self.artist = ArtistState::default();
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

        let tracks = demo::tracks();
        self.covers = demo::covers(&tracks);
        for (index, track) in tracks.iter().enumerate() {
            self.track_palettes
                .insert(track.id.clone(), demo::cover_palette(index));
        }
        self.profile = Some(ProfileData {
            status: demo::status(),
            liked: demo::liked(&tracks),
            total_liked: 248,
            sample: tracks,
        });
        self.track_lyrics
            .insert("1".to_string(), Some(demo::demo_lyrics().to_string()));
        self.restore_state(cx);
        cx.notify();
    }

    pub(crate) fn start_demo_track(&mut self, track: Track, cx: &mut Context<Self>) {
        self.audio.stop();
        self.seek_preview = None;
        self.seek_hover = None;
        self.lyrics_expanded = false;
        self.cache_track(&track);
        self.playback_epoch += 1;
        self.player.track = Some(track.clone());
        self.player.cover = self.covers.get(&track.id).cloned();
        self.player.position = Duration::ZERO;
        self.player.playing = true;
        self.player.loading = false;
        self.player.error = None;
        self.player.quality = Some("MP3 · 320 кбит/с · демо".into());
        self.player.audio_ready = true;
        if matches!(
            self.screen,
            Screen::Track | Screen::NowPlaying | Screen::Wave
        ) {
            self.load_track_backdrop(track, cx);
        }
        self.schedule_state_save(cx);
        cx.notify();
    }
}
