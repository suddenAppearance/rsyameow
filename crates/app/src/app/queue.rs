//! Queue drawer state and lazy queue-track fetching.

use super::*;

impl RootView {
    pub(crate) fn toggle_queue(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.queue_open = !self.queue_open;
        if self.queue_open {
            self.fetch_queue_tracks(cx);
        }
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn close_queue(&mut self, cx: &mut Context<Self>) {
        self.queue_open = false;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn cache_track(&mut self, track: &Track) {
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

    pub(crate) fn fetch_queue_tracks(&mut self, cx: &mut Context<Self>) {
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
    pub(crate) fn ensure_queue(&mut self) -> bool {
        if !self.queue.is_empty() {
            return true;
        }

        self.queue_source = QueueSource::Liked;
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

    pub(crate) fn queue_empty_error(&mut self, cx: &mut Context<Self>) {
        self.player.error = Some(if self.profile.is_none() {
            "Профиль ещё не загружен.".to_string()
        } else if self.demo {
            "В демо-коллекции нет треков.".to_string()
        } else {
            "В «Мне нравится» пока нет треков. Добавьте пару песен в Яндекс Музыке и обновите профиль.".to_string()
        });
        cx.notify();
    }
}
