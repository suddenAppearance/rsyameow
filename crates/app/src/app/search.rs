//! Debounced catalog search and search pagination.

use super::*;

impl RootView {
    pub(crate) fn open_search(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Search;
        self.nav_stack.clear();
        self.queue_open = false;
        self.schedule_state_save(cx);
        let focus = self.search_input.read(cx).focus_handle(cx);
        window.focus(&focus);
        cx.notify();
    }

    pub(crate) fn schedule_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_debounce_epoch = self.search_debounce_epoch.wrapping_add(1);
        let epoch = self.search_debounce_epoch;
        let query = query.trim().to_string();

        if query.is_empty() {
            self.search_epoch = self.search_epoch.wrapping_add(1);
            self.search.query.clear();
            self.search.searched = false;
            self.search.loading = false;
            self.search.error = None;
            self.search.results.clear();
            self.search.artists.clear();
            self.search.albums.clear();
            self.search.total = 0;
            self.search.page = 0;
            self.search.per_page = 0;
            cx.notify();
            return;
        }

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            Timer::after(Duration::from_millis(400)).await;
            this.update(cx, |this, cx| {
                if this.search_debounce_epoch != epoch {
                    return;
                }
                this.run_search(query, 0, cx);
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn perform_search(&mut self, cx: &mut Context<Self>) {
        self.search_debounce_epoch = self.search_debounce_epoch.wrapping_add(1);
        let query = self.search_input.read(cx).text().trim().to_string();
        self.run_search(query, 0, cx);
    }

    pub(crate) fn search_next(&mut self, cx: &mut Context<Self>) {
        if self.search.loading || self.search.query.is_empty() {
            return;
        }
        let next = self.search.page + 1;
        if (next as u64) * self.search.per_page.max(1) >= self.search.total as u64 {
            return;
        }
        let query = self.search.query.clone();
        self.run_search(query, next, cx);
    }

    pub(crate) fn search_previous(&mut self, cx: &mut Context<Self>) {
        if self.search.loading || self.search.page == 0 || self.search.query.is_empty() {
            return;
        }
        let query = self.search.query.clone();
        self.run_search(query, self.search.page - 1, cx);
    }

    pub(crate) fn run_search(&mut self, query: String, page: u32, cx: &mut Context<Self>) {
        if query.is_empty() {
            self.search.searched = true;
            self.search.error = Some("Введите запрос для поиска.".into());
            self.search.loading = false;
            self.search.results.clear();
            self.search.total = 0;
            self.search.page = 0;
            cx.notify();
            return;
        }

        self.search_epoch += 1;
        let epoch = self.search_epoch;
        self.search.query = query.clone();
        self.search.searched = true;
        self.search.loading = true;
        self.search.error = None;
        self.search.page = page;
        self.search.results.clear();
        self.search.total = 0;
        cx.notify();

        if self.demo {
            let needle = query.to_lowercase();
            let total;

            if self.search.kind == SearchKind::Tracks {
                let results: Vec<Track> = self
                    .profile
                    .as_ref()
                    .map(|profile| {
                        profile
                            .sample
                            .iter()
                            .filter(|track| {
                                let haystack = format!(
                                    "{} {} {}",
                                    track.display_title(),
                                    track.artist_names(),
                                    track
                                        .albums
                                        .first()
                                        .and_then(|album| album.title.as_deref())
                                        .unwrap_or_default()
                                )
                                .to_lowercase();
                                haystack.contains(&needle)
                            })
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                total = results.len();
                self.search.results = results.clone();
                self.load_covers(results, cx);
            } else if self.search.kind == SearchKind::Artists {
                let mut seen = HashSet::new();
                let mut artists = Vec::new();
                if let Some(profile) = self.profile.as_ref() {
                    for track in &profile.sample {
                        for artist in &track.artists {
                            if artist
                                .name
                                .as_deref()
                                .is_some_and(|name| name.to_lowercase().contains(&needle))
                                && seen.insert(
                                    artist
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| format!("{:?}", artist.id)),
                                )
                            {
                                artists.push(artist.clone());
                            }
                        }
                    }
                }
                total = artists.len();
                self.search.artists = artists;
            } else {
                let mut seen = HashSet::new();
                let mut albums = Vec::new();
                if let Some(profile) = self.profile.as_ref() {
                    for track in &profile.sample {
                        let artist_names = track.artist_names().to_lowercase();
                        for album in &track.albums {
                            let title = album.title.as_deref().unwrap_or_default();
                            let matches = title.to_lowercase().contains(&needle)
                                || artist_names.contains(&needle);
                            if matches && album.id.is_some_and(|id| seen.insert(id)) {
                                let mut album = album.clone();
                                album.artists = track.artists.clone();
                                albums.push(album);
                            }
                        }
                    }
                }
                total = albums.len();
                self.search.albums = albums;
            }

            self.search.loading = false;
            self.search.total = total;
            self.search.per_page = total.max(1) as u64;
            self.search.page = 0;
            cx.notify();
            return;
        }

        let client = self.client.clone();
        let kind = self.search.kind;
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    match kind {
                        SearchKind::Tracks => client
                            .search_tracks(&query, page)
                            .map(SearchPayload::Tracks),
                        SearchKind::Artists => client
                            .search_artists(&query, page)
                            .map(SearchPayload::Artists),
                        SearchKind::Albums => client
                            .search_albums(&query, page)
                            .map(SearchPayload::Albums),
                    }
                })
                .await;

            this.update(cx, |this, cx| {
                if this.search_epoch != epoch {
                    return;
                }
                this.search.loading = false;
                match result {
                    Ok(payload) => {
                        this.search.error = None;
                        match payload {
                            SearchPayload::Tracks(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.results = found.results;
                                let tracks = this.search.results.clone();
                                this.load_covers(tracks, cx);
                            }
                            SearchPayload::Artists(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.artists = found.results;
                                this.load_search_images(cx);
                            }
                            SearchPayload::Albums(found) => {
                                this.search.total = found.total as usize;
                                this.search.per_page = found.per_page;
                                this.search.albums = found.results;
                                this.load_search_images(cx);
                            }
                        }
                        this.search.page = page;
                    }
                    Err(error) => {
                        this.search.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn load_search_images(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let mut covers = Vec::new();
        for artist in &self.search.artists {
            if let (Some(id), Some(url)) = (artist.id, artist.cover_url(200)) {
                covers.push((format!("artist-{id}"), url));
            }
        }
        for album in &self.search.albums {
            if let (Some(id), Some(url)) = (album.id, album.cover_url(300)) {
                covers.push((format!("album-{id}"), url));
            }
        }
        self.load_entity_images(None, covers, cx);
    }

    pub(crate) fn set_search_kind(&mut self, kind: SearchKind, cx: &mut Context<Self>) {
        if self.search.kind == kind {
            return;
        }
        self.search.kind = kind;
        self.search_debounce_epoch = self.search_debounce_epoch.wrapping_add(1);
        let query = self.search_input.read(cx).text().trim().to_string();
        let query = if query.is_empty() {
            self.search.query.clone()
        } else {
            query
        };
        if query.is_empty() {
            self.search.results.clear();
            self.search.artists.clear();
            self.search.albums.clear();
            self.search.total = 0;
            self.search.page = 0;
            self.search.per_page = 0;
            self.search.searched = false;
            self.search.error = None;
            cx.notify();
            return;
        }
        self.run_search(query, 0, cx);
    }

    pub(crate) fn play_search_result(&mut self, track: Track, cx: &mut Context<Self>) {
        let same_track = self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == track.id);

        if same_track {
            self.toggle_playback(cx);
            return;
        }

        let results = self.search.results.clone();
        for item in &results {
            self.cache_track(item);
        }
        self.queue_source = QueueSource::Search;
        self.queue = results.iter().map(Track::full_id).collect();
        self.queue_index = results.iter().position(|item| item.id == track.id);

        if self.demo {
            self.start_demo_track(track, cx);
        } else {
            let full_id = track.full_id();
            self.load_and_play(Some(track), full_id, cx);
        }
    }
}
