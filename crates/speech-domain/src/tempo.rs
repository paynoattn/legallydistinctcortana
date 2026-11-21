//! Tempo change that keeps the pitch: WSOLA (waveform-similarity overlap-add).
//!
//! The output is built from 30 ms frames of the input, laid down every 15 ms with a Hann
//! window. Frame `k` is taken near `k · 15 ms · speed` in the input, nudged within ±7.5 ms to
//! where the waveform best continues the frame before it, so periods line up and the voice
//! does not warble.

const PER_MILLE: usize = 1_000;
const HOP_PER_MILLE_OF_RATE: u32 = 15;
const MIN_WEIGHT: f32 = 1e-6;
const TWO_PI: f32 = std::f32::consts::TAU;

/// `input` played `speed_per_mille / 1000` times as fast at the same pitch. A speed of 1000
/// returns the input unchanged; 0 is treated as 1.
pub(crate) fn change_tempo(input: &[f32], sample_rate: u32, speed_per_mille: u32) -> Vec<f32> {
    let speed = usize::try_from(speed_per_mille.max(1)).unwrap_or(PER_MILLE);
    if speed == PER_MILLE {
        return input.to_vec();
    }
    let source = Source {
        input,
        hop: hop_length(sample_rate),
    };
    let length = input.len().saturating_mul(PER_MILLE) / speed;
    let mut mix = Mix::silent(length, source.hop);
    let mut start = 0;
    for frame in 0..length.div_ceil(source.hop) {
        mix.add(source.from(start), frame * source.hop);
        let next = frame.saturating_add(1).saturating_mul(source.hop);
        start = source.best_match(start.saturating_add(source.hop), next * speed / PER_MILLE);
    }
    mix.normalized(length)
}

/// Half a frame: 15 ms of samples, at least one.
fn hop_length(sample_rate: u32) -> usize {
    let samples = sample_rate.saturating_mul(HOP_PER_MILLE_OF_RATE) / 1_000;
    usize::try_from(samples.max(1)).unwrap_or(1)
}

/// A periodic Hann window: copies laid every `length / 2` samples add up to 1.
fn hann(length: usize) -> Vec<f32> {
    let size = to_f32(length).max(1.0);
    (0..length)
        .map(|index| 0.5 - 0.5 * (TWO_PI * to_f32(index) / size).cos())
        .collect()
}

fn to_f32(value: usize) -> f32 {
    u16::try_from(value).map_or(f32::from(u16::MAX), f32::from)
}

/// The input, read in frames of two hops.
struct Source<'a> {
    input: &'a [f32],
    hop: usize,
}

impl Source<'_> {
    fn from(&self, start: usize) -> &[f32] {
        self.input.get(start..).unwrap_or_default()
    }

    /// Where near `nominal` the input best continues the samples from `natural` on: the
    /// start, within half a hop either way, whose next hop correlates most with them.
    fn best_match(&self, natural: usize, nominal: usize) -> usize {
        let continuation = self.from(natural);
        let tolerance = self.hop / 2;
        (nominal.saturating_sub(tolerance)..=nominal.saturating_add(tolerance))
            .map(|start| (start, self.similarity(continuation, start)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map_or(nominal, |(start, _)| start)
    }

    fn similarity(&self, continuation: &[f32], start: usize) -> f32 {
        continuation
            .iter()
            .zip(self.from(start))
            .take(self.hop)
            .map(|(a, b)| a * b)
            .sum()
    }
}

/// The overlap-added output and, per sample, the window weight laid over it.
struct Mix {
    window: Vec<f32>,
    sum: Vec<f32>,
    weight: Vec<f32>,
}

impl Mix {
    fn silent(length: usize, hop: usize) -> Self {
        let window = hann(hop.saturating_mul(2));
        let total = length.saturating_add(window.len());
        Self {
            window,
            sum: vec![0.0; total],
            weight: vec![0.0; total],
        }
    }

    fn add(&mut self, frame: &[f32], at: usize) {
        let samples = frame.iter().copied().chain(std::iter::repeat(0.0));
        let slots = self.sum.iter_mut().zip(self.weight.iter_mut()).skip(at);
        for ((sum, weight), (gain, sample)) in slots.zip(self.window.iter().zip(samples)) {
            *sum += sample * gain;
            *weight += gain;
        }
    }

    fn normalized(self, length: usize) -> Vec<f32> {
        self.sum
            .into_iter()
            .zip(self.weight)
            .take(length)
            .map(|(sum, weight)| {
                if weight > MIN_WEIGHT {
                    sum / weight
                } else {
                    0.0
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 22_050;

    fn sine(frequency: f32, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| (TWO_PI * frequency * to_f32(index) / 22_050.0).sin() * 0.5)
            .collect()
    }

    fn rising_zero_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
            .count()
    }

    #[test]
    fn normal_speed_returns_the_input() {
        let input = sine(220.0, 1_000);
        assert_eq!(change_tempo(&input, RATE, 1_000), input, "unchanged at 1×");
    }

    #[test]
    fn double_speed_halves_the_length_and_keeps_the_pitch() {
        let input = sine(220.0, 22_050);
        let output = change_tempo(&input, RATE, 2_000);
        assert_eq!(output.len(), 11_025, "half as long");
        let cycles = rising_zero_crossings(&output);
        assert!(
            (108..=112).contains(&cycles),
            "about 110 cycles in 0.5 s, got {cycles}"
        );
    }

    #[test]
    fn three_quarter_speed_lengthens_and_keeps_the_pitch() {
        let input = sine(220.0, 22_050);
        let output = change_tempo(&input, RATE, 750);
        assert_eq!(output.len(), 29_400, "a third longer");
        let cycles = rising_zero_crossings(&output);
        assert!(
            (290..=296).contains(&cycles),
            "about 293 cycles in 1.33 s, got {cycles}"
        );
    }

    #[test]
    fn a_steady_tone_keeps_its_loudness() {
        let output = change_tempo(&sine(220.0, 22_050), RATE, 1_500);
        let peak = output
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            (0.45..=0.55).contains(&peak),
            "peak stays near 0.5, got {peak}"
        );
    }

    #[test]
    fn silence_stays_silent_and_empty_stays_empty() {
        assert!(
            change_tempo(&vec![0.0; 5_000], RATE, 1_500)
                .iter()
                .all(|s| *s == 0.0),
            "no sound is added"
        );
        assert!(
            change_tempo(&[], RATE, 2_000).is_empty(),
            "nothing in, nothing out"
        );
    }

    #[test]
    fn zero_speed_counts_as_the_slowest_and_a_zero_rate_still_works() {
        assert_eq!(
            change_tempo(&[0.1; 2], RATE, 0).len(),
            2_000,
            "speed 0 is 0.001×"
        );
        assert_eq!(
            change_tempo(&[0.1; 10], 0, 2_000).len(),
            5,
            "hop of one sample"
        );
    }

    #[test]
    fn the_hann_window_overlaps_to_one() {
        let window = hann(8);
        for index in 0..4 {
            let sum = window[index] + window[index + 4];
            assert!((sum - 1.0).abs() < 1e-6, "position {index} sums to {sum}");
        }
        assert_eq!(hop_length(22_050), 330, "15 ms at 22.05 kHz");
    }

    #[test]
    fn best_match_finds_the_start_that_continues_the_waveform() {
        let input = sine(220.0, 4_000);
        let period = 22_050.0 / 220.0;
        let source = Source {
            input: &input,
            hop: 330,
        };
        let found = source.best_match(0, 1_000);
        let phase = to_f32(found) % period;
        assert!(
            phase < 2.0 || phase > period - 2.0,
            "starts in phase with sample 0, got {found}"
        );
    }
}
