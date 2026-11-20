//! Artist pages, previews and full discography loading.

use super::*;

impl RootView {
    pub(crate) fn open_artist(
        &mut self,
        artist_id: u64,
        known: Option<Artist>,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.push_navigation();
        self.screen = Screen::Artist;
        self.artist = ArtistState {
            id: Some(artist_id),
            artist: known.clone(),
            loading: true,
            epoch: self.artist.epoch.wrapping_add(1),
            ..ArtistState::default()
        };
        self.schedule_state_save(cx);
        self.load_artist_data(artist_id, known, cx);
        cx.notify();
    }

    pub(crate) fn load_artist_data(
        &mut self,
        artist_id: u64,
        known: Option<Artist>,
        cx: &mut Context<Self>,
    ) {
        let epoch = self.artist.epoch;

        if self.demo {
            let sample = self
                .profile
                .as_ref()
                .map(|profile| profile.sample.clone())
                .unwrap_or_default();
            let mut tracks = Vec::new();
            let mut albums = Vec::new();
            let mut seen_albums = HashSet::new();
            for track in &sample {
                if track
                    .artists
                    .iter()
                    .any(|artist| artist.id == Some(artist_id))
                {
                    tracks.push(track.clone());
                    for album in &track.albums {
                        if album.id.is_some_and(|id| seen_albums.insert(id)) {
                            let mut album = album.clone();
                            album.artists = track.artists.clone();
                            albums.push(album);
                        }
                    }
                }
            }
            let artist = known
                .or_else(|| {
                    tracks
                        .iter()
                        .flat_map(|track| track.artists.iter())
                        .find(|artist| artist.id == Some(artist_id))
                        .cloned()
                })
                .or_else(|| self.artist.artist.clone());
            self.artist.artist = artist;
            self.artist.tracks = tracks.clone();
            self.artist.albums = albums.clone();
            self.artist.all_tracks = tracks.clone();
            self.artist.all_albums = albums;
            self.artist.loading = false;
            self.artist.error = if self.artist.artist.is_none() {
                Some("Артист не найден в демо-коллекции.".into())
            } else {
                None
            };
            self.load_covers(tracks, cx);
            return;
        }

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let artist = client.artist(artist_id)?;
                    let tracks = client.artist_tracks(artist_id, 0, 50)?;
                    let albums = client.artist_albums(artist_id, 0, 50)?;
                    Ok::<_, MusicError>((artist, tracks.tracks, albums.albums))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.artist.epoch != epoch {
                    return;
                }
                this.artist.loading = false;
                match result {
                    Ok((artist, tracks, albums)) => {
                        this.artist.artist = Some(artist.clone());
                        this.artist.tracks = tracks.clone();
                        this.artist.albums = albums.clone();
                        this.artist.all_tracks = tracks.clone();
                        this.artist.all_albums = albums.clone();
                        this.artist.error = None;
                        this.load_covers(tracks, cx);
                        this.load_entity_images(
                            artist
                                .cover_url(600)
                                .map(|url| (format!("artist-{artist_id}"), url)),
                            albums
                                .iter()
                                .filter_map(|album| {
                                    album.id.and_then(|id| {
                                        album.cover_url(400).map(|url| (format!("album-{id}"), url))
                                    })
                                })
                                .collect(),
                            cx,
                        );
                    }
                    Err(error) => {
                        this.artist.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_artist_tracks(&mut self, cx: &mut Context<Self>) {
        if self.artist.id.is_none() {
            return;
        }
        self.push_navigation();
        self.screen = Screen::ArtistTracks;
        self.queue_open = false;
        self.schedule_state_save(cx);
        self.load_all_artist_tracks(cx);
        cx.notify();
    }

    pub(crate) fn open_artist_albums(&mut self, cx: &mut Context<Self>) {
        if self.artist.id.is_none() {
            return;
        }
        self.push_navigation();
        self.screen = Screen::ArtistAlbums;
        self.queue_open = false;
        self.schedule_state_save(cx);
        self.load_all_artist_albums(cx);
        cx.notify();
    }

    pub(crate) fn load_all_artist_tracks(&mut self, cx: &mut Context<Self>) {
        let Some(artist_id) = self.artist.id else {
            return;
        };
        if self.artist.all_tracks_loading {
            return;
        }
        let total = self
            .artist
            .artist
            .as_ref()
            .and_then(|artist| artist.counts.as_ref().map(|counts| counts.tracks))
            .unwrap_or(0);
        if self.artist.all_tracks.len() as u64 >= total && total > 0 {
            return;
        }
        self.artist.all_tracks_loading = true;
        self.artist.all_tracks_error = None;
        cx.notify();

        if self.demo {
            self.artist.all_tracks = self.artist.tracks.clone();
            self.artist.all_tracks_loading = false;
            cx.notify();
            return;
        }

        let client = self.client.clone();
        let epoch = self.artist.epoch;
        let mut collected = self.artist.all_tracks.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut total = total;
                    for page in ((collected.len() / 50) as u32..).take(20) {
                        let response = client.artist_tracks(artist_id, page, 50)?;
                        if let Some(pager) = response.pager.as_ref() {
                            total = pager.total;
                        }
                        if response.tracks.is_empty() {
                            break;
                        }
                        collected.extend(response.tracks);
                        if total > 0 && collected.len() as u64 >= total {
                            break;
                        }
                    }
                    Ok::<_, MusicError>((collected, total))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.artist.epoch != epoch {
                    return;
                }
                this.artist.all_tracks_loading = false;
                match result {
                    Ok((tracks, _)) => {
                        this.artist.all_tracks_error = None;
                        this.artist.all_tracks = tracks.clone();
                        this.load_covers(tracks, cx);
                    }
                    Err(error) => {
                        this.artist.all_tracks_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn load_all_artist_albums(&mut self, cx: &mut Context<Self>) {
        let Some(artist_id) = self.artist.id else {
            return;
        };
        if self.artist.all_albums_loading {
            return;
        }
        let total = self
            .artist
            .artist
            .as_ref()
            .and_then(|artist| artist.counts.as_ref().map(|counts| counts.direct_albums))
            .unwrap_or(0);
        if self.artist.all_albums.len() as u64 >= total && total > 0 {
            return;
        }
        self.artist.all_albums_loading = true;
        self.artist.all_albums_error = None;
        cx.notify();

        if self.demo {
            self.artist.all_albums = self.artist.albums.clone();
            self.artist.all_albums_loading = false;
            cx.notify();
            return;
        }

        let client = self.client.clone();
        let epoch = self.artist.epoch;
        let mut collected = self.artist.all_albums.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut total = total;
                    for page in ((collected.len() / 50) as u32..).take(20) {
                        let response = client.artist_albums(artist_id, page, 50)?;
                        if let Some(pager) = response.pager.as_ref() {
                            total = pager.total;
                        }
                        if response.albums.is_empty() {
                            break;
                        }
                        collected.extend(response.albums);
                        if total > 0 && collected.len() as u64 >= total {
                            break;
                        }
                    }
                    Ok::<_, MusicError>((collected, total))
                })
                .await;

            this.update(cx, |this, cx| {
                if this.artist.epoch != epoch {
                    return;
                }
                this.artist.all_albums_loading = false;
                match result {
                    Ok((albums, _)) => {
                        let covers = albums
                            .iter()
                            .filter_map(|album| {
                                album.id.and_then(|id| {
                                    album.cover_url(400).map(|url| (format!("album-{id}"), url))
                                })
                            })
                            .collect::<Vec<_>>();
                        this.artist.all_albums_error = None;
                        this.artist.all_albums = albums;
                        this.load_entity_images(None, covers, cx);
                    }
                    Err(error) => {
                        this.artist.all_albums_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
