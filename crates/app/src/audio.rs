//! Thin wrapper around rodio used by the playback UI.

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

/// Owns the output stream and the current player.
///
/// The audio device is opened lazily on the first playback attempt so that a
/// missing or broken sound server does not prevent the application from
/// starting.
pub struct AudioEngine {
    backend: Option<Backend>,
    volume: f32,
    /// Raw bytes of the current track. Retained so a decoder can be rebuilt
    /// when the format's demuxer refuses a backward seek.
    bytes: Option<Arc<[u8]>>,
}

struct Backend {
    // The sink must stay alive for as long as the player is used.
    _sink: MixerDeviceSink,
    player: Player,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            backend: None,
            volume: 0.85,
            bytes: None,
        }
    }

    /// Set the output volume; the value is remembered until a device opens.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(backend) = self.backend.as_ref() {
            backend.player.set_volume(self.volume);
        }
    }

    fn ensure_backend(&mut self) -> Result<&mut Backend, String> {
        if self.backend.is_none() {
            let sink = DeviceSinkBuilder::open_default_sink().map_err(|error| {
                format!(
                    "не удалось открыть аудиоустройство: {error}. \
                     Проверьте PipeWire/PulseAudio или подключите наушники."
                )
            })?;
            let player = Player::connect_new(sink.mixer());
            player.set_volume(self.volume);
            self.backend = Some(Backend {
                _sink: sink,
                player,
            });
        }
        Ok(self.backend.as_mut().expect("backend was just created"))
    }

    /// Decode and play audio bytes (MP3 or FLAC) from the beginning.
    pub fn load(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let bytes: Arc<[u8]> = Arc::from(bytes);
        self.bytes = Some(bytes.clone());
        let backend = self.ensure_backend()?;
        backend.player.stop();
        let decoder = Decoder::try_from(Cursor::new(bytes))
            .map_err(|error| format!("не удалось декодировать аудио: {error}"))?;
        backend.player.append(decoder);
        backend.player.play();
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), String> {
        if let Some(backend) = self.backend.as_mut() {
            backend.player.play();
            Ok(())
        } else {
            Err("трек ещё не загружен".to_string())
        }
    }

    pub fn pause(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            backend.player.pause();
        }
    }

    pub fn stop(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            backend.player.stop();
        }
    }

    pub fn seek(&mut self, position: Duration) -> Result<(), String> {
        let backend = self
            .backend
            .as_mut()
            .ok_or_else(|| "трек ещё не загружен".to_string())?;

        // Fragmented FLAC/MP4 demuxers can only seek forward from their
        // current position. Rebuild the decoder from the start for a backward
        // seek instead of asking Symphonia to move its demuxer backwards.
        if position < backend.player.get_pos() {
            return self.reload_at(position);
        }

        match backend.player.try_seek(position) {
            Ok(()) => Ok(()),
            Err(_) => self.reload_at(position),
        }
    }

    fn reload_at(&mut self, position: Duration) -> Result<(), String> {
        let bytes = self
            .bytes
            .clone()
            .ok_or_else(|| "трек ещё не загружен".to_string())?;
        let backend = self
            .backend
            .as_mut()
            .ok_or_else(|| "трек ещё не загружен".to_string())?;
        let paused = backend.player.is_paused();
        backend.player.stop();
        let decoder = Decoder::try_from(Cursor::new(bytes))
            .map_err(|error| format!("не удалось декодировать аудио: {error}"))?;
        backend.player.append(decoder);
        backend
            .player
            .try_seek(position)
            .map_err(|error| format!("не удалось перемотать трек: {error}"))?;
        if paused {
            backend.player.pause();
        } else {
            backend.player.play();
        }
        Ok(())
    }

    pub fn position(&self) -> Duration {
        self.backend
            .as_ref()
            .map(|backend| backend.player.get_pos())
            .unwrap_or_default()
    }

    /// `true` when the queue has played to the end.
    pub fn is_finished(&self) -> bool {
        self.backend
            .as_ref()
            .map(|backend| backend.player.empty())
            .unwrap_or(false)
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source as _;

    #[test]
    #[ignore = "requires a working audio device"]
    fn opens_default_output_stream() {
        let sink = DeviceSinkBuilder::open_default_sink().expect("open audio device");
        let player = Player::connect_new(sink.mixer());
        let source = rodio::source::SineWave::new(440.0)
            .take_duration(Duration::from_millis(120))
            .amplify(0.05);
        player.append(source);
        player.sleep_until_end();
    }
}
