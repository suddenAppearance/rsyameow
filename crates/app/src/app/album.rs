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

    pub(crate) fn toggle_album_like(&mut self, cx: &mut Context<Self>) {
        let Some(album_id) = self.album.id else {
            return;
        };
        if self.album_like_busy {
            return;
        }
        let Some(profile) = self.profile.as_ref() else {
            return;
        };
        let liked = profile.liked_album_ids.contains(&album_id);

        if self.demo {
            if let Some(profile) = self.profile.as_mut() {
                if liked {
                    profile.liked_album_ids.remove(&album_id);
                } else {
                    profile.liked_album_ids.insert(album_id);
                }
            }
            if liked {
                self.albums.items.retain(|album| album.id != Some(album_id));
            }
            cx.notify();
            return;
        }

        self.album_like_busy = true;
        self.album.error = None;
        let client = self.client.clone();
        let epoch = self.profile_epoch;
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.like_albums(&[album_id], !liked) })
                .await;
            this.update(cx, |this, cx| {
                if this.profile_epoch != epoch {
                    return;
                }
                this.album_like_busy = false;
                match result {
                    Ok(()) => {
                        if let Some(profile) = this.profile.as_mut() {
                            if liked {
                                profile.liked_album_ids.remove(&album_id);
                            } else {
                                profile.liked_album_ids.insert(album_id);
                            }
                        }
                        if liked {
                            this.albums.items.retain(|album| album.id != Some(album_id));
                        } else if let Some(album) = this.album.album.clone()
                            && !this.albums.items.iter().any(|item| item.id == album.id)
                        {
                            this.albums.items.insert(0, album);
                        }
                    }
                    Err(error) => this.album.error = Some(error_text(&error)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
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
                    .update(cx, |this, _| {
                        this.search_images.contains_key(&key)
                            && this.track_palettes.contains_key(&key)
                            && this.track_backdrops.contains_key(&key)
                    })
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
                let palette = cover_palette(&bytes);
                let backdrop = super::cover_art::blurred_cover(&bytes);
                let Some(image) = decode_image(bytes) else {
                    continue;
                };
                let keep_going = this
                    .update(cx, |this, cx| {
                        if this.profile_epoch != epoch {
                            return false;
                        }
                        if let Some(palette) = palette {
                            this.track_palettes.insert(key.clone(), palette);
                        }
                        if let Some(backdrop) = backdrop {
                            this.track_backdrops.insert(key.clone(), backdrop);
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
