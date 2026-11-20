//! Thin wrapper around rodio used by the playback UI.

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::io::Cursor;
use std::time::Duration;

/// Owns the output stream and the current player.
///
/// The audio device is opened lazily on the first playback attempt so that a
/// missing or broken sound server does not prevent the application from
/// starting.
pub struct AudioEngine {
    backend: Option<Backend>,
}

struct Backend {
    // The sink must stay alive for as long as the player is used.
    _sink: MixerDeviceSink,
    player: Player,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { backend: None }
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
            player.set_volume(0.85);
            self.backend = Some(Backend {
                _sink: sink,
                player,
            });
        }
        Ok(self.backend.as_mut().expect("backend was just created"))
    }

    /// Decode and play MP3 bytes from the beginning.
    pub fn load(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let backend = self.ensure_backend()?;
        backend.player.stop();
        let decoder = Decoder::try_from(Cursor::new(bytes))
            .map_err(|error| format!("не удалось декодировать MP3: {error}"))?;
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
        backend
            .player
            .try_seek(position)
            .map_err(|error| format!("не удалось перемотать трек: {error}"))
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
