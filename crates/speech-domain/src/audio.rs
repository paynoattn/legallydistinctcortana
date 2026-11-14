use std::time::Duration;

use thiserror::Error;

use crate::dsp;

const PER_MILLE: u32 = 1_000;
const MILLIS_PER_SECOND: u64 = 1_000;
const I16_SCALE: f32 = 32_768.0;
const BYTES_PER_SAMPLE: usize = 2;
const DECIBELS_PER_DECADE: f32 = 20.0;

/// Highest accepted sample rate. Real voices and devices use 8 to 192 kHz.
pub const MAX_SAMPLE_RATE: u32 = 384_000;

/// Why raw audio could not become an [`AudioClip`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AudioError {
    /// A sample rate of zero.
    #[error("sample rate must be above zero")]
    ZeroSampleRate,
    /// A sample rate no audio device or voice uses, which would make derived buffers huge.
    #[error("sample rate {rate} Hz is above the {MAX_SAMPLE_RATE} Hz limit")]
    SampleRateTooHigh {
        /// The rejected rate.
        rate: u32,
    },
    /// Zero channels.
    #[error("audio must have at least one channel")]
    NoChannels,
    /// The byte count is not a whole number of 16-bit frames.
    #[error("{bytes} bytes is not a whole number of 16-bit frames of {channels} channel(s)")]
    PartialFrame {
        /// Bytes received.
        bytes: usize,
        /// Channels per frame.
        channels: u16,
    },
}

/// Mono audio: samples in `-1.0..=1.0` at a fixed sample rate.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioClip {
    sample_rate: u32,
    samples: Vec<f32>,
}

impl AudioClip {
    /// A clip of `samples` at `sample_rate` Hz.
    ///
    /// # Errors
    ///
    /// [`AudioError::ZeroSampleRate`].
    pub fn from_samples(sample_rate: u32, samples: Vec<f32>) -> Result<Self, AudioError> {
        if sample_rate == 0 {
            return Err(AudioError::ZeroSampleRate);
        }
        if sample_rate > MAX_SAMPLE_RATE {
            return Err(AudioError::SampleRateTooHigh { rate: sample_rate });
        }
        Ok(Self {
            sample_rate,
            samples,
        })
    }

    /// Decodes interleaved little-endian signed 16-bit PCM, averaging channels to mono.
    ///
    /// # Errors
    ///
    /// [`AudioError::ZeroSampleRate`], [`AudioError::NoChannels`], or
    /// [`AudioError::PartialFrame`] when `bytes` ends mid-frame.
    pub fn from_pcm16(sample_rate: u32, channels: u16, bytes: &[u8]) -> Result<Self, AudioError> {
        if channels == 0 {
            return Err(AudioError::NoChannels);
        }
        let frame_bytes = BYTES_PER_SAMPLE * usize::from(channels);
        if !bytes.len().is_multiple_of(frame_bytes) {
            return Err(AudioError::PartialFrame {
                bytes: bytes.len(),
                channels,
            });
        }
        let samples = bytes
            .chunks_exact(frame_bytes)
            .map(|frame| mono_sample(frame, channels))
            .collect();
        Self::from_samples(sample_rate, samples)
    }

    /// Samples per second.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The mono samples.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// How long the clip plays, rounded down to the millisecond.
    #[must_use]
    pub fn duration(&self) -> Duration {
        let samples = u64::try_from(self.samples.len()).unwrap_or(u64::MAX);
        Duration::from_millis(
            samples.saturating_mul(MILLIS_PER_SECOND) / u64::from(self.sample_rate.max(1)),
        )
    }

    /// Consumes the clip, returning its samples.
    #[must_use]
    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    /// The same sound at another sample rate (linear interpolation).
    #[must_use]
    pub fn resampled(&self, sample_rate: u32) -> Self {
        let step = dsp::step(self.sample_rate, sample_rate.max(1));
        Self {
            sample_rate: sample_rate.max(1),
            samples: dsp::stretch(&self.samples, step),
        }
    }

    /// Each sample averaged with the `window - 1` samples before it: a simple low-pass filter
    /// for before downsampling, so sound above the new rate's range does not fold back as
    /// noise. A window of 0 or 1 leaves the clip as it is.
    #[must_use]
    pub fn smoothed(&self, window: u16) -> Self {
        let width = usize::from(window.max(1));
        let samples = (0..self.samples.len())
            .map(|end| {
                let start = (end + 1).saturating_sub(width);
                mean(self.samples.get(start..=end).unwrap_or_default())
            })
            .collect();
        Self {
            sample_rate: self.sample_rate,
            samples,
        }
    }

    /// Played `speed_per_mille / 1000` times as fast (pitch moves with speed).
    #[must_use]
    pub fn sped_up(&self, speed_per_mille: u32) -> Self {
        Self {
            sample_rate: self.sample_rate,
            samples: dsp::stretch(&self.samples, dsp::step(speed_per_mille, PER_MILLE)),
        }
    }

    /// Louder or quieter by `decibels`, clipped to `-1.0..=1.0`.
    #[must_use]
    pub fn with_gain(&self, decibels: i8) -> Self {
        let factor = 10_f32.powf(f32::from(decibels) / DECIBELS_PER_DECADE);
        Self {
            sample_rate: self.sample_rate,
            samples: self
                .samples
                .iter()
                .map(|sample| (sample * factor).clamp(-1.0, 1.0))
                .collect(),
        }
    }

    /// The clip followed by `millis` of silence.
    #[must_use]
    pub fn with_trailing_silence(&self, millis: u32) -> Self {
        let count = u64::from(self.sample_rate) * u64::from(millis) / MILLIS_PER_SECOND;
        let silence = usize::try_from(count).unwrap_or(0);
        let mut samples = self.samples.clone();
        samples.resize(samples.len() + silence, 0.0);
        Self {
            sample_rate: self.sample_rate,
            samples,
        }
    }
}

fn mean(window: &[f32]) -> f32 {
    let count = u16::try_from(window.len()).map_or(f32::from(u16::MAX), f32::from);
    window.iter().sum::<f32>() / count.max(1.0)
}

fn mono_sample(frame: &[u8], channels: u16) -> f32 {
    let (samples, _) = frame.as_chunks::<BYTES_PER_SAMPLE>();
    let sum: f32 = samples
        .iter()
        .map(|bytes| f32::from(i16::from_le_bytes(*bytes)) / I16_SCALE)
        .sum();
    sum / f32::from(channels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(rate: u32, samples: &[f32]) -> AudioClip {
        AudioClip::from_samples(rate, samples.to_vec()).unwrap()
    }

    #[test]
    fn duration_is_the_sample_count_over_the_rate_in_whole_milliseconds() {
        assert_eq!(
            clip(16_000, &[0.0; 4_000]).duration(),
            Duration::from_millis(250)
        );
        assert_eq!(clip(1_000, &[0.0; 3]).duration(), Duration::from_millis(3));
        assert_eq!(clip(3_000, &[0.0; 2]).duration(), Duration::ZERO);
        assert_eq!(clip(3_000, &[0.0; 5]).duration(), Duration::from_millis(1));
        assert_eq!(clip(48_000, &[]).duration(), Duration::ZERO);
    }

    #[test]
    fn smoothing_averages_each_sample_with_the_ones_before_it() {
        let smoothed = clip(48_000, &[0.3, 0.6, 0.9, 0.0, -0.9]).smoothed(3);
        assert_eq!(smoothed.sample_rate(), 48_000);
        let expected = [0.3, 0.45, 0.6, 0.5, 0.0];
        assert_eq!(smoothed.samples().len(), expected.len());
        for (got, want) in smoothed.samples().iter().zip(expected) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
    }

    #[test]
    fn smoothing_over_one_sample_or_none_changes_nothing() {
        let original = clip(16_000, &[0.5, -0.25, 1.0]);
        assert_eq!(original.smoothed(1), original);
        assert_eq!(original.smoothed(0), original);
        assert_eq!(clip(16_000, &[]).smoothed(3).samples(), [0.0_f32; 0]);
    }

    #[test]
    fn smoothing_a_tone_at_the_window_length_cancels_it() {
        let tone = clip(48_000, &[1.0, -0.5, -0.5, 1.0, -0.5, -0.5]).smoothed(3);
        assert!(
            tone.samples()[2..].iter().all(|sample| sample.abs() < 1e-6),
            "{:?}",
            tone.samples()
        );
    }

    #[test]
    fn rejects_a_zero_sample_rate() {
        assert_eq!(
            AudioClip::from_samples(0, vec![]),
            Err(AudioError::ZeroSampleRate)
        );
    }

    #[test]
    fn accepts_the_highest_sample_rate_and_rejects_one_more() {
        assert_eq!(MAX_SAMPLE_RATE, 384_000);
        assert_eq!(
            AudioClip::from_samples(384_000, vec![])
                .unwrap()
                .sample_rate(),
            384_000
        );
        assert_eq!(
            AudioClip::from_samples(384_001, vec![]),
            Err(AudioError::SampleRateTooHigh { rate: 384_001 })
        );
        assert_eq!(
            AudioClip::from_pcm16(u32::MAX, 1, &[0, 0]),
            Err(AudioError::SampleRateTooHigh { rate: u32::MAX })
        );
    }

    #[test]
    fn the_sample_rate_limit_is_explained_to_people() {
        assert_eq!(
            AudioError::SampleRateTooHigh { rate: 384_001 }.to_string(),
            "sample rate 384001 Hz is above the 384000 Hz limit"
        );
    }

    #[test]
    fn decodes_mono_pcm16() {
        let bytes = [0x00, 0x40, 0x00, 0xC0, 0xFF, 0x7F];
        let decoded = AudioClip::from_pcm16(22_050, 1, &bytes).unwrap();
        assert_eq!(decoded.sample_rate(), 22_050);
        assert_eq!(decoded.samples(), [0.5, -0.5, 32_767.0 / 32_768.0]);
    }

    #[test]
    fn averages_stereo_frames_to_mono() {
        let bytes = [0x00, 0x40, 0x00, 0x00];
        let decoded = AudioClip::from_pcm16(16_000, 2, &bytes).unwrap();
        assert_eq!(decoded.into_samples(), [0.25]);
    }

    #[test]
    fn rejects_partial_frames_and_zero_channels() {
        assert_eq!(
            AudioClip::from_pcm16(16_000, 2, &[0, 0, 0]),
            Err(AudioError::PartialFrame {
                bytes: 3,
                channels: 2
            })
        );
        assert_eq!(
            AudioClip::from_pcm16(16_000, 0, &[]),
            Err(AudioError::NoChannels)
        );
    }

    #[test]
    fn error_messages_are_readable() {
        assert_eq!(
            AudioError::PartialFrame {
                bytes: 3,
                channels: 2
            }
            .to_string(),
            "3 bytes is not a whole number of 16-bit frames of 2 channel(s)"
        );
        assert_eq!(
            AudioError::ZeroSampleRate.to_string(),
            "sample rate must be above zero"
        );
        assert_eq!(
            AudioError::NoChannels.to_string(),
            "audio must have at least one channel"
        );
    }

    #[test]
    fn resampling_to_double_rate_doubles_the_samples() {
        let up = clip(8_000, &[0.0, 1.0]).resampled(16_000);
        assert_eq!(up.sample_rate(), 16_000);
        assert_eq!(up.samples(), [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn speeding_up_shortens_and_slowing_down_lengthens() {
        let source = clip(1_000, &[0.0; 1_000]);
        assert_eq!(source.sped_up(2_000).samples().len(), 500);
        assert_eq!(source.sped_up(500).samples().len(), 2_000);
        assert_eq!(source.sped_up(1_000).sample_rate(), 1_000);
    }

    #[test]
    fn gain_scales_and_clips() {
        let louder = clip(1_000, &[0.25, -0.9]).with_gain(6);
        let samples = louder.samples();
        assert!(
            samples.first().is_some_and(|s| (s - 0.498_8).abs() < 0.001),
            "+6 dB roughly doubles: {samples:?}"
        );
        assert_eq!(samples.get(1), Some(&-1.0));
        assert_eq!(louder.sample_rate(), 1_000);
        assert_eq!(clip(1_000, &[0.5]).with_gain(0).samples(), [0.5]);
    }

    #[test]
    fn trailing_silence_appends_zeros_for_the_duration() {
        let padded = clip(1_000, &[0.5]).with_trailing_silence(3);
        assert_eq!(padded.samples(), [0.5, 0.0, 0.0, 0.0]);
        assert_eq!(padded.sample_rate(), 1_000);
    }
}
