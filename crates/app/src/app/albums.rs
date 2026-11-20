//! Loading of the albums saved in the user's collection.

use super::*;

impl RootView {
    pub(crate) fn open_albums(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Albums;
        self.queue_open = false;
        self.nav_stack.clear();
        if !self.albums.loaded && !self.albums.loading {
            self.load_albums(cx);
        }
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn load_albums(&mut self, cx: &mut Context<Self>) {
        if self.albums.loading {
            return;
        }
        self.albums.epoch = self.albums.epoch.wrapping_add(1);
        let epoch = self.albums.epoch;
        self.albums.loading = true;
        self.albums.error = None;
        cx.notify();

        if self.demo {
            let albums = demo::albums();
            let first_tracks = albums
                .iter()
                .filter_map(|album| album.volumes.first())
                .filter_map(|volume| volume.first().cloned())
                .collect::<Vec<_>>();
            self.albums.items = albums;
            self.albums.loading = false;
            self.albums.loaded = true;
            self.load_covers(first_tracks, cx);
            cx.notify();
            return;
        }

        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.liked_albums() })
                .await;

            this.update(cx, |this, cx| {
                if this.albums.epoch != epoch {
                    return;
                }
                this.albums.loading = false;
                this.albums.loaded = true;
                match result {
                    Ok(albums) => {
                        this.albums.items = albums;
                        this.albums.error = None;
                        let covers = this
                            .albums
                            .items
                            .iter()
                            .filter_map(|album| {
                                album.id.and_then(|id| {
                                    album.cover_url(400).map(|url| (format!("album-{id}"), url))
                                })
                            })
                            .collect::<Vec<_>>();
                        this.load_entity_images(None, covers, cx);
                    }
                    Err(error) => {
                        this.albums.error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
