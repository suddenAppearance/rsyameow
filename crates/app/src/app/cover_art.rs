//! Covers, palette extraction, blurred backdrops and track preloading.

use super::*;

impl RootView {
    pub(crate) fn load_covers(&mut self, tracks: Vec<Track>, cx: &mut Context<Self>) {
        let epoch = self.profile_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            for track in tracks {
                let already_loaded = this
                    .update(cx, |this, _| this.covers.contains_key(&track.id))
                    .unwrap_or(true);
                if already_loaded {
                    continue;
                }

                let client = client.clone();
                let track_id = track.id.clone();
                let bytes = cx
                    .background_spawn(
                        async move { client.download_cover(&track, 400).ok().flatten() },
                    )
                    .await;

                let Some(bytes) = bytes else {
                    continue;
                };
                let palette = cover_palette(&bytes);
                let Some(image) = decode_image(bytes) else {
                    continue;
                };

                let still_current = this
                    .update(cx, |this, cx| {
                        if this.profile_epoch != epoch {
                            return false;
                        }
                        if let Some(palette) = palette {
                            this.track_palettes.insert(track_id.clone(), palette);
                        }
                        let is_current = this
                            .player
                            .track
                            .as_ref()
                            .is_some_and(|track| track.id == track_id);
                        this.covers.insert(track_id.clone(), image.clone());
                        if is_current {
                            this.player.cover = Some(image);
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !still_current {
                    break;
                }
            }
        })
        .detach();
    }

    /// Download audio in the background so the next transition can reuse the
    /// bytes without waiting for `/download-info` and the file itself.
    pub(crate) fn prefetch_track(&mut self, track: Track, cx: &mut Context<Self>) {
        if self.demo || self.client.token().is_none() {
            return;
        }
        let id = track.id.clone();
        if self.preloaded.contains_key(&id) || self.prefetching.contains(&id) {
            return;
        }
        if self
            .player
            .track
            .as_ref()
            .is_some_and(|current| current.id == id)
            && self.player.audio_ready
        {
            return;
        }

        self.prefetching.insert(id.clone());
        self.load_covers(vec![track.clone()], cx);
        let epoch = self.profile_epoch;
        let client = self.client.clone();
        let audio_quality = self.audio_quality;
        let request_id = id.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let result = cx
                .background_spawn(async move {
                    client.download_track_with_quality(&request_id, audio_quality)
                })
                .await;
            this.update(cx, |this, _cx| {
                this.prefetching.remove(&id);
                if this.profile_epoch != epoch {
                    return;
                }
                if let Ok(downloaded) = result {
                    while this.preloaded.len() >= 4 {
                        let Some(oldest) = this.preloaded.keys().find(|key| **key != id).cloned()
                        else {
                            break;
                        };
                        this.preloaded.remove(&oldest);
                    }
                    this.preloaded.insert(id, downloaded);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Yandex-yellow fallback palette for tracks without a cover.
    pub(crate) fn track_palette(&self, track_id: &str) -> (Rgba, Rgba) {
        self.track_palettes
            .get(track_id)
            .copied()
            .unwrap_or_else(default_palette)
    }

    pub(crate) fn track_backdrop(&self, track_id: &str) -> Option<Arc<Image>> {
        self.track_backdrops.get(track_id).cloned()
    }

    pub(crate) fn load_track_backdrop(&mut self, track: Track, cx: &mut Context<Self>) {
        let id = track.id.clone();
        if self.track_backdrops.contains_key(&id) || self.backdrop_loading.contains(&id) {
            return;
        }
        let Some(url) = track.cover_url(600) else {
            return;
        };
        self.backdrop_loading.insert(id.clone());
        let epoch = self.profile_epoch;
        let client = self.client.clone();

        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            let bytes = cx
                .background_spawn(async move { client.download_image(&url).ok() })
                .await;
            let Some(bytes) = bytes else {
                let _ = this.update(cx, |this, _| {
                    this.backdrop_loading.remove(&id);
                });
                return;
            };
            let palette = cover_palette(&bytes);
            let backdrop = blurred_cover(&bytes);
            this.update(cx, |this, cx| {
                this.backdrop_loading.remove(&id);
                if this.profile_epoch != epoch {
                    return;
                }
                if let Some(palette) = palette {
                    this.track_palettes.insert(id.clone(), palette);
                }
                if let Some(backdrop) = backdrop {
                    this.track_backdrops.insert(id, backdrop);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// A Yandex yellow gradient palette used when a track has no cover.
pub(crate) fn default_palette() -> (Rgba, Rgba) {
    (
        Rgba {
            r: 0.10,
            g: 0.085,
            b: 0.025,
            a: 1.0,
        },
        Rgba {
            r: 1.0,
            g: 0.84,
            b: 0.24,
            a: 1.0,
        },
    )
}

/// Average the saturated pixels of a cover into a dark base and a bright glow.
pub(crate) fn cover_palette(bytes: &[u8]) -> Option<(Rgba, Rgba)> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    let mut samples: Vec<(f32, f32, f32, f32)> = Vec::new();

    for y in (0..height).step_by(4) {
        for x in (0..width).step_by(4) {
            let pixel = image.get_pixel(x, y);
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };
            if saturation > 0.14 && max > 0.12 {
                samples.push((saturation, r, g, b));
            }
        }
    }

    if samples.is_empty() {
        return Some(default_palette());
    }

    samples.sort_by(|left, right| right.0.total_cmp(&left.0));
    let take = (samples.len() / 8).clamp(1, 64);
    let mut red = 0.0f32;
    let mut green = 0.0f32;
    let mut blue = 0.0f32;
    for (_, r, g, b) in samples.iter().take(take) {
        red += r;
        green += g;
        blue += b;
    }
    let red = red / take as f32;
    let green = green / take as f32;
    let blue = blue / take as f32;

    let average = Rgba {
        r: red,
        g: green,
        b: blue,
        a: 1.0,
    };
    let mut hue: Hsla = average.into();
    if hue.s < 0.08 {
        return Some(default_palette());
    }
    hue.s = (hue.s * 1.25 + 0.22).min(0.96);
    let glow: Rgba = Hsla {
        h: hue.h,
        s: hue.s,
        l: 0.6,
        a: 1.0,
    }
    .into();
    let base: Rgba = Hsla {
        h: hue.h,
        s: (hue.s * 0.78).min(0.92),
        l: 0.115,
        a: 1.0,
    }
    .into();
    Some((base, glow))
}

/// Create a small, softly blurred version of the cover for immersive pages.
pub(crate) fn blurred_cover(bytes: &[u8]) -> Option<Arc<Image>> {
    let source = image::load_from_memory(bytes).ok()?.to_rgba8();
    let small = image::imageops::resize(&source, 64, 64, image::imageops::FilterType::Triangle);
    let blurred = image::imageops::blur(&small, 12.0);
    let large = image::imageops::resize(&blurred, 512, 512, image::imageops::FilterType::Triangle);
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(large)
        .write_to(&mut png, image::ImageFormat::Png)
        .ok()?;
    Some(Arc::new(Image::from_bytes(
        gpui::ImageFormat::Png,
        png.into_inner(),
    )))
}

/// Decode downloaded cover bytes into a gpui image.
pub(crate) fn decode_image(bytes: Vec<u8>) -> Option<Arc<Image>> {
    let format = match image::guess_format(&bytes).ok()? {
        image::ImageFormat::Png => gpui::ImageFormat::Png,
        image::ImageFormat::Jpeg => gpui::ImageFormat::Jpeg,
        image::ImageFormat::WebP => gpui::ImageFormat::Webp,
        image::ImageFormat::Gif => gpui::ImageFormat::Gif,
        image::ImageFormat::Bmp => gpui::ImageFormat::Bmp,
        image::ImageFormat::Tiff => gpui::ImageFormat::Tiff,
        _ => return None,
    };

    Some(Arc::new(Image::from_bytes(format, bytes)))
}

/// A short Russian explanation for API errors.
pub(crate) fn error_text(error: &MusicError) -> String {
    match error {
        MusicError::Network(_) | MusicError::Timeout => {
            "Нет связи с Яндекс Музыкой. Проверьте интернет и попробуйте ещё раз.".into()
        }
        MusicError::Unauthorized(_) => "Яндекс отклонил токен. Войдите в аккаунт заново.".into(),
        MusicError::NotFound(_) => "Трек или аккаунт не найден.".into(),
        MusicError::NoDownload => {
            "Для этого трека нет доступного MP3. Возможно, он доступен только с подпиской Plus."
                .into()
        }
        MusicError::DeviceAuth(message) => {
            format!("Яндекс не подтвердил вход: {message}")
        }
        MusicError::Api { status, .. } => {
            format!("Яндекс Музыка вернула ошибку (HTTP {status}). Попробуйте позже.")
        }
        MusicError::NotAuthenticated => "Нужно войти в аккаунт.".into(),
        MusicError::BadRequest(_) | MusicError::Decode(_) => {
            "Сервис вернул неожиданный ответ. Попробуйте ещё раз.".into()
        }
        MusicError::UnknownAccount => "Не удалось определить аккаунт.".into(),
    }
}
