//! Personal wave loading, continuation and prefetching.

use super::*;

impl RootView {
    /// Keep the previous and next wave tracks warm. The current track is
    /// already being streamed (or was prefetched on restore).
    pub(crate) fn prefetch_wave_neighbors(&mut self, cx: &mut Context<Self>) {
        if self.demo || self.queue_source != QueueSource::Wave {
            return;
        }
        let Some(index) = self.queue_index else {
            return;
        };
        let len = self.queue.len();
        if len < 2 {
            return;
        }

        let mut targets = Vec::new();
        if let Some(previous) = self.queue.get(step_index(Some(index), len, false)) {
            targets.push(previous.clone());
        }
        if index + 1 < len
            && let Some(next) = self.queue.get(index + 1)
        {
            targets.push(next.clone());
        }

        for queue_id in targets {
            if let Some(track) = self.queue_track_cache.get(&queue_id).cloned() {
                self.prefetch_track(track, cx);
            }
        }
    }

    // ------------------------------------------------------------- Cover mood

    pub(crate) fn fresh_wave_tracks(
        incoming: Vec<Track>,
        existing: &[Track],
        queue: Option<&[String]>,
    ) -> Vec<Track> {
        let mut seen: HashSet<String> = HashSet::new();
        for track in existing {
            seen.insert(track.id.clone());
            seen.insert(track.full_id());
        }
        if let Some(queue) = queue {
            for id in queue {
                seen.insert(id.clone());
                seen.insert(id.split(':').next().unwrap_or(id).to_string());
            }
        }

        incoming
            .into_iter()
            .filter(|track| {
                let id = track.id.clone();
                let full_id = track.full_id();
                if seen.contains(&id) || seen.contains(&full_id) {
                    false
                } else {
                    seen.insert(id);
                    seen.insert(full_id);
                    true
                }
            })
            .collect()
    }

    pub(crate) fn open_wave(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Wave;
        if let Some(track) = self.player.track.clone() {
            self.load_track_backdrop(track, cx);
        }
        self.prefetch_wave_neighbors(cx);
        self.nav_stack.clear();
        self.queue_open = false;
        if !self.wave.loaded && !self.wave.loading {
            self.load_wave(false, false, cx);
        }
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn load_wave(&mut self, append: bool, autoplay: bool, cx: &mut Context<Self>) {
        if self.wave.loading || self.wave.fetching {
            return;
        }
        self.wave.epoch = self.wave.epoch.wrapping_add(1);
        let epoch = self.wave.epoch;
        let append = append && !self.wave.tracks.is_empty();
        let old_queue_len = self.queue.len();
        self.wave.error = None;
        if append {
            self.wave.fetching = true;
        } else {
            self.wave.loading = true;
        }
        cx.notify();

        if self.demo {
            let sample = self
                .profile
                .as_ref()
                .map(|profile| profile.sample.clone())
                .unwrap_or_default();
            let incoming: Vec<Track> = if append {
                sample.iter().rev().cloned().collect()
            } else {
                sample.clone()
            };
            let existing = self.wave.tracks.clone();
            let incoming = Self::fresh_wave_tracks(
                incoming,
                &existing,
                if append { Some(&self.queue) } else { None },
            );
            if incoming.is_empty() {
                self.wave.loading = false;
                self.wave.fetching = false;
                self.wave.error = Some(if append {
                    "В демо-коллекции больше нет новых треков.".into()
                } else {
                    "Демо-коллекция пуста.".into()
                });
                cx.notify();
                return;
            }
            if append {
                self.wave.tracks.extend(incoming.clone());
            } else {
                self.wave.tracks = incoming.clone();
            }
            self.wave.loaded = true;
            self.wave.loading = false;
            self.wave.fetching = false;
            for track in &incoming {
                self.cache_track(track);
            }
            self.load_covers(incoming.clone(), cx);

            if autoplay {
                if append {
                    for track in &incoming {
                        self.cache_track(track);
                        self.queue.push(track.full_id());
                    }
                    self.queue_source = QueueSource::Wave;
                    self.play_queue_index(old_queue_len, cx);
                } else {
                    self.start_wave_queue(0, cx);
                }
            }
            self.schedule_state_save(cx);
            cx.notify();
            return;
        }

        let queue = if append {
            self.wave
                .tracks
                .last()
                .map(|track| track.id.clone())
                .or_else(|| {
                    self.queue
                        .last()
                        .map(|id| id.split(':').next().unwrap_or(id).to_string())
                })
        } else {
            None
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.wave_tracks(queue.as_deref()) })
                .await;

            this.update(cx, |this, cx| {
                if this.wave.epoch != epoch {
                    return;
                }
                this.wave.loading = false;
                this.wave.fetching = false;
                match result {
                    Ok(batch) => {
                        let incoming = Self::fresh_wave_tracks(
                            batch.tracks(),
                            &this.wave.tracks,
                            if append { Some(&this.queue) } else { None },
                        );
                        if incoming.is_empty() {
                            this.wave.error = Some(if append {
                                "Волна вернула только уже знакомые треки. Попробуйте ещё раз."
                                    .into()
                            } else {
                                "Волна не вернула треки. Попробуйте ещё раз.".into()
                            });
                        } else {
                            this.wave.loaded = true;
                            this.wave.error = None;
                            if append {
                                for track in &incoming {
                                    this.cache_track(track);
                                    this.queue.push(track.full_id());
                                }
                                this.wave.tracks.extend(incoming.clone());
                                if autoplay {
                                    this.queue_source = QueueSource::Wave;
                                    this.play_queue_index(old_queue_len, cx);
                                }
                            } else {
                                this.wave.tracks = incoming.clone();
                                for track in &incoming {
                                    this.cache_track(track);
                                }
                                if autoplay {
                                    this.start_wave_queue(0, cx);
                                }
                            }
                            this.load_covers(incoming, cx);
                            this.prefetch_wave_neighbors(cx);
                        }
                    }
                    Err(error) => {
                        this.wave.error = Some(error_text(&error));
                        if autoplay {
                            this.player.error = Some(error_text(&error));
                        }
                    }
                }
                this.schedule_state_save(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn start_wave_queue(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.wave.tracks.is_empty() {
            return;
        }
        let tracks = self.wave.tracks.clone();
        for track in &tracks {
            self.cache_track(track);
        }
        self.queue_source = QueueSource::Wave;
        self.queue = tracks.iter().map(Track::full_id).collect();
        self.queue_index = None;
        self.play_queue_index(index.min(self.queue.len().saturating_sub(1)), cx);
        self.schedule_state_save(cx);
    }

    pub(crate) fn play_wave(&mut self, cx: &mut Context<Self>) {
        if self.wave.tracks.is_empty() {
            self.load_wave(false, true, cx);
        } else {
            self.start_wave_queue(0, cx);
        }
    }
}
