//! Album page loading and playback.

use super::*;

impl RootView {
    pub(crate) fn open_album(
        &mut self,
        album_id: u64,
        known: Option<Album>,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.push_navigation();
        self.screen = Screen::Album;
        self.album = AlbumState {
            id: Some(album_id),
            album: known.clone(),
            loading: true,
            epoch: self.album.epoch.wrapping_add(1),
            ..AlbumState::default()
        };
        self.schedule_state_save(cx);
        self.load_album_data(album_id, known, cx);
        cx.notify();
    }

    pub(crate) fn load_album_data(
        &mut self,
        album_id: u64,
        known: Option<Album>,
        cx: &mut Context<Self>,
    ) {
        let epoch = self.album.epoch;

        if self.demo {
            let sample = self
                .profile
                .as_ref()
                .map(|profile| profile.sample.clone())
                .unwrap_or_default();
            let tracks: Vec<Track> = sample
                .iter()
                .filter(|track| track.albums.iter().any(|album| album.id == Some(album_id)))
                .cloned()
                .collect();
            let mut album = known.or_else(|| self.album.album.clone());
            if album.is_none()
                && let Some(track) = tracks.first()
                && let Some(base) = track.albums.first()
            {
                let mut value = base.clone();
                value.artists = track.artists.clone();
                album = Some(value);
            }
            if let Some(album) = &mut album {
                album.volumes = vec![tracks.clone()];
            }
            self.album.album = album;
            self.album.tracks = tracks.clone();
            self.album.loading = false;
            self.album.error = if self.album.album.is_none() {
                Some("Альбом не найден в демо-коллекции.".into())
            } else {
                None
            };
            self.load_covers(tracks, cx);
            return;
        }

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.album_with_tracks(album_id) })
                .await;

            this.update(cx, |this, cx| {
                if this.album.epoch != epoch {
                    return;
                }
                this.album.loading = false;
                match result {
                    Ok(album) => {
                        let tracks = album.all_tracks();
                        this.album.album = Some(album.clone());
                        this.album.tracks = tracks.clone();
                        this.album.error = None;
                        this.load_covers(tracks, cx);
                        this.load_entity_images(
                            album
                                .cover_url(600)
                                .map(|url| (format!("album-{album_id}"), url)),
                            Vec::new(),
                            cx,
                        );
                    }
                    Err(error) => {
                        this.album.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn load_entity_images(
        &mut self,
        single: Option<(String, String)>,
        many: Vec<(String, String)>,
        cx: &mut Context<Self>,
    ) {
        let mut covers = Vec::new();
        if let Some(cover) = single {
            covers.push(cover);
        }
        covers.extend(many);
        if covers.is_empty() {
            return;
        }
        let epoch = self.profile_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            for (key, url) in covers {
                let already = this
                    .update(cx, |this, _| this.search_images.contains_key(&key))
                    .unwrap_or(true);
                if already {
                    continue;
                }
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
                        if this.profile_epoch != epoch {
                            return false;
                        }
                        this.search_images.insert(key, image);
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
}
