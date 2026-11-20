//! OAuth device flow, token login and logout.

use super::*;

impl RootView {
    pub(crate) fn start_device_flow(&mut self, cx: &mut Context<Self>) {
        if self.auth_busy || self.device.is_some() {
            return;
        }

        self.auth_busy = true;
        self.login_error = None;
        self.device = None;
        self.auth_epoch += 1;
        let epoch = self.auth_epoch;
        let client = self.client.clone();
        cx.notify();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move { client.request_device_code() })
                .await;

            this.update(cx, |this, cx| {
                if this.auth_epoch != epoch {
                    return;
                }
                this.auth_busy = false;
                match result {
                    Ok(code) => {
                        this.device = Some(DeviceFlow {
                            code,
                            copied: false,
                        });
                        this.spawn_device_polling(cx);
                    }
                    Err(error) => {
                        this.login_error = Some(error_text(&error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn spawn_device_polling(&mut self, cx: &mut Context<Self>) {
        let Some(flow) = &self.device else {
            return;
        };
        let device_code = flow.code.device_code.clone();
        let interval = flow.code.interval.max(2);
        let expires_in = flow.code.expires_in.max(60);
        let epoch = self.auth_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let deadline = Instant::now() + Duration::from_secs(expires_in);

            loop {
                Timer::after(Duration::from_secs(interval)).await;

                let active = this
                    .update(cx, |this, _| {
                        this.auth_epoch == epoch && this.device.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }

                if Instant::now() >= deadline {
                    this.update(cx, |this, cx| {
                        if this.auth_epoch == epoch {
                            this.device = None;
                            this.login_error =
                                Some("Код подтверждения истёк. Получите новый код.".into());
                            cx.notify();
                        }
                    })
                    .ok();
                    break;
                }

                let client = client.clone();
                let code = device_code.clone();
                let result = cx
                    .background_spawn(async move { client.poll_device_token(&code) })
                    .await;

                match result {
                    Ok(Some(token)) => {
                        let _ = this.update(cx, |this, cx| {
                            if this.auth_epoch == epoch {
                                this.on_authenticated(token.access_token, true, cx);
                            }
                        });
                        break;
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        this.update(cx, |this, cx| {
                            if this.auth_epoch == epoch {
                                this.device = None;
                                this.login_error = Some(error_text(&error));
                                cx.notify();
                            }
                        })
                        .ok();
                        break;
                    }
                }
            }
        })
        .detach();
    }

    pub(crate) fn cancel_device_flow(&mut self, cx: &mut Context<Self>) {
        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        cx.notify();
    }

    pub(crate) fn copy_device_code(&mut self, cx: &mut Context<Self>) {
        let Some(flow) = self.device.as_mut() else {
            return;
        };
        let code = flow.code.user_code.clone();
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        flow.copied = true;
        cx.notify();
    }

    pub(crate) fn begin_login_with_token(
        &mut self,
        token: impl Into<String>,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        let token = token.into().trim().to_string();
        if token.is_empty() {
            self.login_error = Some("Введите OAuth-токен.".into());
            self.show_token_input = true;
            cx.notify();
            return;
        }

        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.demo = false;
        self.pending_save_token = remember.then(|| token.clone());
        self.client.set_token(token.clone());
        self.load_profile(cx);
    }

    pub(crate) fn on_authenticated(
        &mut self,
        token: String,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        self.auth_epoch += 1;
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.pending_save_token = remember.then(|| token.clone());
        self.client.set_token(token.clone());
        self.load_profile(cx);
    }

    pub(crate) fn logout(&mut self, cx: &mut Context<Self>) {
        let was_demo = self.demo;
        self.auth_epoch += 1;
        self.profile_epoch += 1;
        self.playback_epoch += 1;
        self.audio.stop();
        self.client.clear_token();
        if !was_demo {
            // Demo mode does not own the saved session, so leaving it must not
            // delete the user's token.
            config::clear_token();
        }

        self.screen = Screen::Login;
        self.demo = false;
        self.profile = None;
        self.covers.clear();
        self.queue.clear();
        self.queue_index = None;
        self.queue_open = false;
        self.queue_track_cache.clear();
        self.queue_fetching = false;
        self.like_busy = false;
        self.player.reset();
        self.device = None;
        self.auth_busy = false;
        self.login_error = None;
        self.show_token_input = false;
        self.pending_save_token = None;
        self.search = SearchState::default();
        self.search_epoch += 1;
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
        self.token_input.update(cx, |input, cx| input.clear(cx));
        self.search_input.update(cx, |input, cx| input.clear(cx));
        config::clear_app_state();
        cx.notify();
    }
}
