use kernel::Mood;

use crate::AudioClip;

/// How a mood is approximated on a voice with no emotion control: speed, loudness, and the
/// pause after each sentence. Speed is applied by resampling, so pitch rises with speed.
///
/// Gains never exceed 0 dB: synthesizers such as Piper already peak near full scale, so the
/// loudest mood (Mad) plays at the voice's own level and every other mood is quieter. Neutral
/// sits 6 dB below, which leaves the loud moods their headroom instead of clipping them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prosody {
    /// Playback speed in thousandths (1000 = unchanged).
    pub speed_per_mille: u32,
    /// Loudness change in decibels.
    pub gain_db: i8,
    /// Silence after each sentence, in milliseconds.
    pub pause_ms: u32,
}

const fn prosody(speed_per_mille: u32, gain_db: i8, pause_ms: u32) -> Prosody {
    Prosody {
        speed_per_mille,
        gain_db,
        pause_ms,
    }
}

const NEUTRAL: Prosody = prosody(1_000, -6, 250);

const TABLE: [(Mood, Prosody); 9] = [
    (Mood::Neutral, NEUTRAL),
    (Mood::Calm, prosody(920, -8, 420)),
    (Mood::Cheerful, prosody(1_070, -5, 220)),
    (Mood::Alert, prosody(1_100, -3, 160)),
    (Mood::Urgent, prosody(1_180, -2, 100)),
    (Mood::Angry, prosody(1_120, -1, 140)),
    (Mood::Mad, prosody(1_250, 0, 80)),
    (Mood::Whisper, prosody(940, -16, 360)),
    (Mood::Deadpan, prosody(960, -7, 320)),
];

impl Prosody {
    /// The approximation used for `mood`.
    #[must_use]
    pub fn for_mood(mood: Mood) -> Self {
        TABLE
            .iter()
            .find_map(|(entry, prosody)| (*entry == mood).then_some(*prosody))
            .unwrap_or(NEUTRAL)
    }

    /// `clip` shaped by this prosody: sped up or slowed down, louder or quieter, then paused.
    #[must_use]
    pub fn apply(&self, clip: &AudioClip) -> AudioClip {
        clip.sped_up(self.speed_per_mille)
            .with_gain(self.gain_db)
            .with_trailing_silence(self.pause_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_keeps_the_speed_and_leaves_six_decibels_of_headroom() {
        let clip = AudioClip::from_samples(1_000, vec![0.5; 10]).unwrap();
        let shaped = Prosody::for_mood(Mood::Neutral).apply(&clip);
        assert_eq!(shaped.samples().len(), 10 + 250);
        assert!(
            shaped
                .samples()
                .first()
                .is_some_and(|s| (s - 0.250_6).abs() < 0.001),
            "-6 dB roughly halves the level: {:?}",
            shaped.samples().first()
        );
    }

    #[test]
    fn no_mood_amplifies_so_full_scale_speech_never_clips() {
        for mood in Mood::ALL {
            assert!(
                Prosody::for_mood(mood).gain_db <= 0,
                "{mood} must not push audio past full scale"
            );
        }
    }

    #[test]
    fn urgent_moods_are_faster_and_louder_than_calm_ones() {
        let neutral = Prosody::for_mood(Mood::Neutral);
        let calm = Prosody::for_mood(Mood::Calm);
        let whisper = Prosody::for_mood(Mood::Whisper);
        for loud in [Mood::Alert, Mood::Urgent, Mood::Angry, Mood::Mad] {
            let prosody = Prosody::for_mood(loud);
            assert!(
                prosody.speed_per_mille > calm.speed_per_mille,
                "{loud} is faster than calm"
            );
            assert!(
                prosody.gain_db > neutral.gain_db,
                "{loud} is louder than neutral"
            );
            assert!(
                prosody.pause_ms < calm.pause_ms,
                "{loud} pauses less than calm"
            );
        }
        assert!(whisper.gain_db < calm.gain_db, "whisper is the quietest");
    }

    #[test]
    fn mad_is_the_most_extreme_mood() {
        let mad = Prosody::for_mood(Mood::Mad);
        for mood in Mood::ALL {
            let other = Prosody::for_mood(mood);
            assert!(
                mad.speed_per_mille >= other.speed_per_mille,
                "mad is at least as fast as {mood}"
            );
            assert!(
                mad.gain_db >= other.gain_db,
                "mad is at least as loud as {mood}"
            );
        }
    }

    #[test]
    fn every_mood_has_a_documented_setting() {
        let table: Vec<(u32, i8, u32)> = Mood::ALL
            .into_iter()
            .map(Prosody::for_mood)
            .map(|p| (p.speed_per_mille, p.gain_db, p.pause_ms))
            .collect();
        assert_eq!(
            table,
            [
                (1_000, -6, 250),
                (920, -8, 420),
                (1_070, -5, 220),
                (1_100, -3, 160),
                (1_180, -2, 100),
                (1_120, -1, 140),
                (1_250, 0, 80),
                (940, -16, 360),
                (960, -7, 320),
            ]
        );
    }

    #[test]
    fn apply_speeds_up_and_keeps_the_level_for_mad() {
        let clip = AudioClip::from_samples(1_000, vec![0.1; 1_250]).unwrap();
        let shaped = Prosody::for_mood(Mood::Mad).apply(&clip);
        assert_eq!(shaped.samples().len(), 1_000 + 80);
        assert_eq!(shaped.samples().first(), Some(&0.1));
    }
}
