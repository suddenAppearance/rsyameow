//! MPRIS media controls integration.

use super::*;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use std::sync::mpsc::{self, Receiver};

pub(crate) fn init_media_controls() -> (Option<MediaControls>, Receiver<MediaControlEvent>) {
    let (sender, receiver) = mpsc::channel();
    let has_session_bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| std::path::Path::new(&dir).join("bus").exists())
            .unwrap_or(false);
    if !has_session_bus {
        return (None, receiver);
    }
    let mut controls = match MediaControls::new(PlatformConfig {
        display_name: "Я.Мяу",
        dbus_name: "rsyameow",
        hwnd: None,
    }) {
        Ok(controls) => controls,
        Err(_) => return (None, receiver),
    };
    if controls
        .attach(move |event| {
            let _ = sender.send(event);
        })
        .is_err()
    {
        return (None, receiver);
    }
    (Some(controls), receiver)
}

impl RootView {
    pub(crate) fn handle_media_event(&mut self, event: MediaControlEvent, cx: &mut Context<Self>) {
        match event {
            MediaControlEvent::Play => {
                if !self.player.playing {
                    self.toggle_playback(cx);
                }
            }
            MediaControlEvent::Pause | MediaControlEvent::Stop => {
                if self.player.playing {
                    self.toggle_playback(cx);
                }
            }
            MediaControlEvent::Toggle => self.toggle_playback(cx),
            MediaControlEvent::Next => self.play_next(cx),
            MediaControlEvent::Previous => self.play_previous(cx),
            MediaControlEvent::SetPosition(MediaPosition(position)) => {
                if let Some(duration) = self.player.track.as_ref().map(Track::duration)
                    && !duration.is_zero()
                {
                    self.seek_to(
                        (position.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0),
                        cx,
                    );
                }
            }
            MediaControlEvent::SeekBy(direction, amount) => {
                let Some(duration) = self.player.track.as_ref().map(Track::duration) else {
                    return;
                };
                if duration.is_zero() {
                    return;
                }
                let target = match direction {
                    SeekDirection::Forward => self.player.position.saturating_add(amount),
                    SeekDirection::Backward => self.player.position.saturating_sub(amount),
                }
                .min(duration);
                self.seek_to(target.as_secs_f32() / duration.as_secs_f32(), cx);
            }
            MediaControlEvent::SetVolume(volume) => {
                self.set_volume(volume.clamp(0.0, 1.0) as f32, cx);
            }
            MediaControlEvent::Raise => {
                let _ = self.window_handle.update(cx, |_, window, _| {
                    window.activate_window();
                });
            }
            MediaControlEvent::Quit => {
                let _ = self.window_handle.update(cx, |_, window, _| {
                    window.remove_window();
                });
            }
            _ => {}
        }
    }

    pub(crate) fn sync_media_controls(&mut self) {
        let Some(controls) = self.media_controls.as_mut() else {
            return;
        };
        let track = self.player.track.clone();
        let key = track
            .as_ref()
            .map(|track| track.id.clone())
            .unwrap_or_default();
        let track_changed = self.media_track_key.as_deref() != Some(key.as_str());

        if track_changed {
            self.media_track_key = Some(key);
            if let Some(track) = &track {
                let title = track.display_title().to_string();
                let artist = track.artist_names();
                let album = track
                    .albums
                    .first()
                    .map(|album| album.display_title().to_string())
                    .unwrap_or_default();
                let cover = track.cover_url(600);
                let _ = controls.set_metadata(MediaMetadata {
                    title: Some(&title),
                    artist: Some(&artist),
                    album: Some(&album),
                    cover_url: cover.as_deref(),
                    duration: Some(track.duration()),
                });
            } else {
                let _ = controls.set_metadata(MediaMetadata::default());
            }
        }

        let playing = self.player.playing;
        if self.media_playing != Some(playing) {
            self.media_playing = Some(playing);
            let progress = if track.is_some() {
                Some(MediaPosition(self.player.position))
            } else {
                None
            };
            let state = if track.is_none() {
                MediaPlayback::Stopped
            } else if playing {
                MediaPlayback::Playing { progress }
            } else {
                MediaPlayback::Paused { progress }
            };
            let _ = controls.set_playback(state);
        }

        if self.media_volume != Some(self.volume) {
            self.media_volume = Some(self.volume);
            let _ = controls.set_volume(f64::from(self.volume));
        }
    }
}
