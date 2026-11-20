//! Thin wrapper around rodio used by the playback UI.

use crate::config::EqualizerPreset;
use rodio::source::{SeekError, Source};
use rodio::{ChannelCount, Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate};
use std::f64::consts::PI;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

type DecoderSource = Decoder<Cursor<Arc<[u8]>>>;
type EqualizedDecoder = EqualizedSource<DecoderSource>;

/// Owns the output stream and the current player.
///
/// The audio device is opened lazily on the first playback attempt so that a
/// missing or broken sound server does not prevent the application from
/// starting.
pub struct AudioEngine {
    backend: Option<Backend>,
    volume: f32,
    equalizer: EqualizerPreset,
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
            equalizer: EqualizerPreset::Flat,
            bytes: None,
        }
    }

    /// Map the linear UI slider to a quadratic gain curve. This makes quiet
    /// steps near the top of the slider less extreme and attenuation in the
    /// lower half much more noticeable.
    fn volume_gain(volume: f32) -> f32 {
        let volume = volume.clamp(0.0, 1.0);
        volume * volume
    }

    /// Set the output volume; the value is remembered until a device opens.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(backend) = self.backend.as_ref() {
            backend.player.set_volume(Self::volume_gain(self.volume));
        }
    }

    /// Change the equalizer preset. The current track is rebuilt in place so
    /// the effect applies immediately.
    pub fn set_equalizer(&mut self, preset: EqualizerPreset) {
        if self.equalizer == preset {
            return;
        }
        self.equalizer = preset;
        if self.backend.is_some() && self.bytes.is_some() {
            let position = self.position();
            let _ = self.reload_at(position);
        }
    }

    fn make_decoder(
        bytes: Arc<[u8]>,
        equalizer: EqualizerPreset,
    ) -> Result<EqualizedDecoder, String> {
        let decoder = Decoder::try_from(Cursor::new(bytes))
            .map_err(|error| format!("не удалось декодировать аудио: {error}"))?;
        Ok(EqualizedSource::new(decoder, equalizer))
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
            player.set_volume(Self::volume_gain(self.volume));
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
        let equalizer = self.equalizer;
        let backend = self.ensure_backend()?;
        backend.player.stop();
        let decoder = Self::make_decoder(bytes, equalizer)?;
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
        let equalizer = self.equalizer;
        let backend = self
            .backend
            .as_mut()
            .ok_or_else(|| "трек ещё не загружен".to_string())?;
        let paused = backend.player.is_paused();
        backend.player.stop();
        let decoder = Self::make_decoder(bytes, equalizer)?;
        backend.player.append(decoder);
        // A freshly appended decoder is already at the start. Asking
        // Symphonia for a zero-position seek can still return an error even
        // though playback is correctly rewound, so skip that redundant seek.
        if !position.is_zero() {
            backend
                .player
                .try_seek(position)
                .map_err(|error| format!("не удалось перемотать трек: {error}"))?;
        }
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

struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn peaking(sample_rate: f64, frequency: f64, q: f64, gain_db: f64) -> Self {
        let frequency = frequency.clamp(20.0, sample_rate * 0.45);
        let a = 10f64.powf(gain_db / 40.0);
        let omega = 2.0 * PI * frequency / sample_rate;
        let alpha = omega.sin() / (2.0 * q);
        let cos = omega.cos();
        let a0 = 1.0 + alpha / a;

        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let x = f64::from(input);
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y as f32
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

struct EqualizedSource<S> {
    inner: S,
    filters: Vec<[Biquad; 5]>,
    channel: usize,
}

impl<S: Source<Item = f32>> EqualizedSource<S> {
    fn new(inner: S, preset: EqualizerPreset) -> Self {
        let sample_rate = f64::from(inner.sample_rate().get());
        let channels = inner.channels().get() as usize;
        let gains = match preset {
            EqualizerPreset::Flat => [0.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Bass => [6.0, 4.0, 1.0, 0.0, 0.0],
            EqualizerPreset::Vocal => [-3.0, 0.0, 3.0, 4.0, 1.0],
            EqualizerPreset::Rock => [4.0, 2.0, -2.0, 2.0, 4.0],
            EqualizerPreset::Electronic => [5.0, 2.0, 0.0, 3.0, 5.0],
        };
        let bands = [60.0, 230.0, 910.0, 3600.0, 14000.0];
        let channel_filters = (0..channels.max(1))
            .map(|_| {
                std::array::from_fn(|index| {
                    Biquad::peaking(sample_rate, bands[index], 1.0, gains[index])
                })
            })
            .collect::<Vec<_>>();
        Self {
            inner,
            filters: channel_filters,
            channel: 0,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for EqualizedSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let channel = self.channel % self.filters.len().max(1);
        self.channel = (self.channel + 1) % self.filters.len().max(1);
        let mut output = sample;
        for filter in &mut self.filters[channel] {
            output = filter.process(output);
        }
        Some(output.clamp(-1.0, 1.0))
    }
}

impl<S: Source<Item = f32>> Source for EqualizedSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(position)?;
        for filters in &mut self.filters {
            for filter in filters {
                filter.reset();
            }
        }
        Ok(())
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

    #[test]
    fn volume_uses_a_quadratic_curve() {
        assert!((AudioEngine::volume_gain(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((AudioEngine::volume_gain(0.5) - 0.25).abs() < f32::EPSILON);
        assert!((AudioEngine::volume_gain(0.85) - 0.7225).abs() < 0.0001);
        assert!((AudioEngine::volume_gain(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((AudioEngine::volume_gain(2.0) - 1.0).abs() < f32::EPSILON);
    }

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
