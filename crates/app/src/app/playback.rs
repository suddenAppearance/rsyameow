//! Queue management, audio loading, seeking and transport controls.

use super::*;

impl RootView {
    pub(crate) fn play_context_track(
        &mut self,
        track: Track,
        tracks: Vec<Track>,
        cx: &mut Context<Self>,
    ) {
        if tracks.is_empty() {
            return;
        }
        for item in &tracks {
            self.cache_track(item);
        }
        self.queue_source = QueueSource::Artist;
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = tracks.iter().position(|item| item.id == track.id);
        let index = self.queue_index.unwrap_or_else(|| {
            tracks
                .iter()
                .position(|item| item.id == track.id)
                .unwrap_or(0)
        });
        self.play_queue_index(index, cx);
        self.schedule_state_save(cx);
    }

    pub(crate) fn play_context_all(
        &mut self,
        tracks: Vec<Track>,
        shuffle: bool,
        cx: &mut Context<Self>,
    ) {
        if tracks.is_empty() {
            self.player.error = Some("Здесь нет треков.".into());
            cx.notify();
            return;
        }
        for item in &tracks {
            self.cache_track(item);
        }
        self.queue_source = QueueSource::Artist;
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = None;
        let index = if shuffle {
            rand::rng().random_range(0..tracks.len())
        } else {
            0
        };
        self.play_queue_index(index, cx);
        self.schedule_state_save(cx);
    }

    // ---------------------------------------------------------------- Playlists

    pub(crate) fn play_queue_index(&mut self, index: usize, cx: &mut Context<Self>) {
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

        let known = self.queue_track_cache.get(&id).cloned().or_else(|| {
            profile
                .sample
                .iter()
                .find(|track| track.id == id || track.full_id() == id)
                .cloned()
        });
        if let Some(track) = &known {
            self.cache_track(track);
        }
        self.load_and_play(known, id, cx);
        self.prefetch_wave_neighbors(cx);
    }

    pub(crate) fn set_liked_queue(&mut self) {
        self.queue_source = QueueSource::Liked;
        let mut known = Vec::new();
        self.queue = self
            .profile
            .as_ref()
            .map(|profile| {
                known = profile.sample.clone();
                if self.demo {
                    profile
                        .sample
                        .iter()
                        .map(|track| track.id.clone())
                        .collect()
                } else {
                    profile.liked.iter().map(TrackShort::track_id).collect()
                }
            })
            .unwrap_or_default();
        for track in &known {
            self.cache_track(track);
        }
    }

    pub(crate) fn play_random(&mut self, cx: &mut Context<Self>) {
        if self.player.loading {
            return;
        }

        self.set_liked_queue();
        if self.queue.is_empty() {
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

        if self.queue_source == QueueSource::Wave
            && self
                .queue_index
                .is_some_and(|index| index + 1 >= self.queue.len())
            && !self.wave.loading
            && !self.wave.fetching
        {
            self.load_wave(true, true, cx);
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
        self.queue_source = QueueSource::Liked;
        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);

        if same_track {
            self.toggle_playback(cx);
            return;
        }

        self.set_liked_queue();
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

    pub(crate) fn load_and_play(
        &mut self,
        track: Option<Track>,
        requested_id: String,
        cx: &mut Context<Self>,
    ) {
        self.load_and_play_at(track, requested_id, Duration::ZERO, cx);
    }

    pub(crate) fn load_and_play_at(
        &mut self,
        track: Option<Track>,
        requested_id: String,
        start_at: Duration,
        cx: &mut Context<Self>,
    ) {
        if self.client.token().is_none() {
            self.player.error = Some("Нужно войти в аккаунт.".into());
            cx.notify();
            return;
        }

        self.audio.stop();
        self.seek_preview = None;
        self.seek_hover = None;
        self.seek_controls_hovered = false;
        self.playback_epoch += 1;
        let epoch = self.playback_epoch;
        let client = self.client.clone();
        let audio_quality = self.audio_quality;
        let known = track.clone();
        let cache_id = track
            .as_ref()
            .map(|track| track.id.clone())
            .unwrap_or_else(|| {
                requested_id
                    .split(':')
                    .next()
                    .unwrap_or(&requested_id)
                    .to_string()
            });
        let cached_audio = self.preloaded.remove(&cache_id);
        let have_cover = track
            .as_ref()
            .is_some_and(|track| self.covers.contains_key(&track.id));

        if let Some(track) = &track {
            self.lyrics_expanded = false;
            self.player.track = Some(track.clone());
            self.player.cover = self.covers.get(&track.id).cloned();
        }
        self.player.position = start_at;
        self.player.playing = false;
        self.player.loading = true;
        self.player.error = None;
        self.player.quality = None;
        self.player.audio_ready = false;
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let track = match known {
                        Some(track) => track,
                        None => client.track(&requested_id)?,
                    };
                    let downloaded = match cached_audio {
                        Some(downloaded) => downloaded,
                        None => client.download_track_with_quality(&track.id, audio_quality)?,
                    };
                    let cover = if have_cover {
                        None
                    } else {
                        client.download_cover(&track, 600).ok().flatten()
                    };
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
                        if let Some(bytes) = cover {
                            if let Some(palette) = cover_palette(&bytes) {
                                this.track_palettes.insert(track.id.clone(), palette);
                            }
                            if let Some(image) = decode_image(bytes) {
                                this.covers.insert(track.id.clone(), image.clone());
                                this.player.cover = Some(image);
                            }
                        }

                        this.cache_track(&track);
                        this.player.track = Some(track.clone());
                        this.player.position = start_at.min(track.duration());
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
                                let target = start_at.min(track.duration());
                                if !target.is_zero() {
                                    if let Err(error) = this.audio.seek(target) {
                                        this.player.error = Some(error);
                                    } else {
                                        this.player.position = target;
                                    }
                                }
                                this.player.audio_ready = true;
                                this.player.playing = true;
                            }
                            Err(error) => {
                                this.player.playing = false;
                                this.player.audio_ready = false;
                                this.player.error = Some(error);
                            }
                        }
                        if matches!(
                            this.screen,
                            Screen::Track | Screen::NowPlaying | Screen::Wave
                        ) {
                            this.load_track_backdrop(track.clone(), cx);
                        }
                    }
                    Err(error) => {
                        this.player.playing = false;
                        this.player.error = Some(error_text(&error));
                    }
                }

                this.schedule_state_save(cx);
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

    pub(crate) fn set_audio_quality(
        &mut self,
        quality: yandex_music::AudioQuality,
        cx: &mut Context<Self>,
    ) {
        self.audio_quality = quality;
        self.save_playback_settings();
        cx.notify();
    }

    pub(crate) fn set_equalizer(
        &mut self,
        equalizer: config::EqualizerPreset,
        cx: &mut Context<Self>,
    ) {
        self.equalizer_commit_epoch = self.equalizer_commit_epoch.wrapping_add(1);
        self.equalizer = equalizer;
        self.equalizer_gains = equalizer.gains();
        self.audio.set_equalizer(self.equalizer_gains);
        self.save_playback_settings();
        cx.notify();
    }

    pub(crate) fn set_equalizer_band(&mut self, index: usize, gain: f32, cx: &mut Context<Self>) {
        if index >= self.equalizer_gains.len() {
            return;
        }
        self.equalizer_gains[index] = gain.clamp(-12.0, 12.0);
        self.equalizer = config::EqualizerPreset::Custom;
        self.schedule_equalizer_commit(cx);
        cx.notify();
    }

    fn schedule_equalizer_commit(&mut self, cx: &mut Context<Self>) {
        self.equalizer_commit_epoch = self.equalizer_commit_epoch.wrapping_add(1);
        let epoch = self.equalizer_commit_epoch;
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            Timer::after(Duration::from_millis(250)).await;
            this.update(cx, |this, cx| {
                if this.equalizer_commit_epoch != epoch {
                    return;
                }
                this.audio.set_equalizer(this.equalizer_gains);
                this.save_playback_settings();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn begin_equalizer_drag(&mut self, index: usize, gain: f32, cx: &mut Context<Self>) {
        self.eq_drag = Some(index);
        self.set_equalizer_band(index, gain, cx);
    }

    pub(crate) fn drag_equalizer(&mut self, index: usize, gain: f32, cx: &mut Context<Self>) {
        if self.eq_drag == Some(index) {
            self.set_equalizer_band(index, gain, cx);
        }
    }

    pub(crate) fn end_equalizer_drag(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.eq_drag == Some(index) {
            self.eq_drag = None;
            self.commit_equalizer(cx);
        }
    }

    pub(crate) fn commit_equalizer(&mut self, cx: &mut Context<Self>) {
        self.equalizer_commit_epoch = self.equalizer_commit_epoch.wrapping_add(1);
        self.audio.set_equalizer(self.equalizer_gains);
        self.save_playback_settings();
        cx.notify();
    }

    fn save_playback_settings(&self) {
        let settings = config::PlaybackSettings {
            volume: self.volume,
            quality: self.audio_quality,
            equalizer: self.equalizer,
            equalizer_gains: Some(self.equalizer_gains),
        };
        if let Err(error) = config::save_playback_settings(&settings) {
            eprintln!("failed to save playback settings: {error}");
        }
    }

    pub(crate) fn seek_to(&mut self, fraction: f32, cx: &mut Context<Self>) {
        self.seek_preview = None;
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

        if !self.player.audio_ready {
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
        self.schedule_state_save(cx);
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

        if !self.player.audio_ready {
            let start_at = self.player.position;
            let id = track.full_id();
            self.load_and_play_at(Some(track), id, start_at, cx);
            return;
        }

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

/// Advance a queue position with wrap-around.
pub(crate) fn step_index(current: Option<usize>, len: usize, forward: bool) -> usize {
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
