use std::time::Duration;

/// How long the check stays after text is inserted, before the mark fades out.
pub const INSERTED_LINGER: Duration = Duration::from_millis(700);

/// How long an error stays on screen unless it is clicked first.
pub const ERROR_LINGER: Duration = Duration::from_secs(6);

/// Input level, in dB below full scale, at which the mark starts to react.
pub const QUIET_DB: f32 = -50.0;

/// Input level, in dB below full scale, at which the mark is fully lit.
pub const LOUD_DB: f32 = -10.0;

/// What the corner indicator shows. See `docs/architecture/voice-and-speech.md#overlay-states`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayState {
    /// Nothing on screen.
    Hidden,
    /// The dictation key is down, or the speech is being turned into text.
    Dictating(Stage),
    /// The text was typed into the focused app.
    Inserted,
    /// Reading aloud.
    Speaking,
    /// Something failed; the one place the indicator can be clicked.
    Error(OverlayError),
}

/// How far a dictation has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The dictation key went down; the microphone is opening.
    Armed,
    /// Recording; the mark follows the input level.
    Listening,
    /// The key went up; the speech is being transcribed and typed.
    Thinking,
}

/// A failure shown on the indicator: one line, plus how to fix it when there is a way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayError {
    /// One short sentence for the person using the app.
    pub reason: String,
    /// What clicking the fix link does, if anything.
    pub fix: Option<Fix>,
}

/// An action offered next to an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fix {
    /// Open the system sound settings to choose a microphone.
    SoundSettings,
    /// Open the app's settings (for example, to change the voice server).
    Settings,
}

/// Something that happened, which may change what the indicator shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayEvent {
    /// The dictation key went down.
    KeyDown,
    /// The microphone opened and is recording.
    Recording,
    /// The dictation key went up.
    KeyUp,
    /// The dictated text was typed.
    Inserted,
    /// The dictation ended with nothing to type (a tap, or silence).
    Discarded,
    /// Something failed.
    Failed(OverlayError),
    /// Reading aloud started.
    SpeakingStarted,
    /// Reading aloud finished or was stopped.
    SpeakingEnded,
    /// The current state's [`OverlayState::linger`] time ran out.
    Elapsed,
    /// The person clicked the error away.
    Dismissed,
}

impl OverlayState {
    /// The state after `event`. Events that do not apply to the current state leave it as
    /// it is: for example, key repeats while listening, or a reading that ends during a
    /// dictation.
    #[must_use]
    pub fn after(&self, event: OverlayEvent) -> Self {
        match event {
            OverlayEvent::Failed(error) => Self::Error(error),
            OverlayEvent::KeyDown => self.pressed(),
            OverlayEvent::SpeakingStarted => self.reading(),
            OverlayEvent::Recording
            | OverlayEvent::KeyUp
            | OverlayEvent::Inserted
            | OverlayEvent::Discarded => self.dictated(&event),
            OverlayEvent::SpeakingEnded | OverlayEvent::Elapsed | OverlayEvent::Dismissed => {
                self.closed(&event)
            }
        }
    }

    fn pressed(&self) -> Self {
        match self {
            Self::Dictating(Stage::Listening | Stage::Thinking) => self.clone(),
            _ => Self::Dictating(Stage::Armed),
        }
    }

    fn reading(&self) -> Self {
        match self {
            Self::Dictating(_) => self.clone(),
            _ => Self::Speaking,
        }
    }

    fn dictated(&self, event: &OverlayEvent) -> Self {
        let Self::Dictating(stage) = self else {
            return self.clone();
        };
        match (stage, event) {
            (Stage::Armed, OverlayEvent::Recording) => Self::Dictating(Stage::Listening),
            (Stage::Armed | Stage::Listening, OverlayEvent::KeyUp) => {
                Self::Dictating(Stage::Thinking)
            }
            (Stage::Thinking, OverlayEvent::Inserted) => Self::Inserted,
            (_, OverlayEvent::Discarded) => Self::Hidden,
            _ => self.clone(),
        }
    }

    fn closed(&self, event: &OverlayEvent) -> Self {
        match (self, event) {
            (Self::Speaking, OverlayEvent::SpeakingEnded)
            | (Self::Inserted | Self::Error(_), OverlayEvent::Elapsed)
            | (Self::Error(_), OverlayEvent::Dismissed) => Self::Hidden,
            _ => self.clone(),
        }
    }

    /// How long this state stays before an [`OverlayEvent::Elapsed`] should be sent, or
    /// `None` when it stays until something else happens.
    #[must_use]
    pub const fn linger(&self) -> Option<Duration> {
        match self {
            Self::Inserted => Some(INSERTED_LINGER),
            Self::Error(_) => Some(ERROR_LINGER),
            _ => None,
        }
    }

    /// Whether the indicator is on screen.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }

    /// Whether the indicator takes mouse clicks. Only errors do; every other state lets
    /// clicks through to the window underneath.
    #[must_use]
    pub const fn takes_clicks(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// The state's name for the indicator window: `hidden`, `armed`, `listening`,
    /// `thinking`, `inserted`, `speaking`, or `error`.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Dictating(stage) => stage.id(),
            Self::Inserted => "inserted",
            Self::Speaking => "speaking",
            Self::Error(_) => "error",
        }
    }

    /// This state, or [`OverlayState::Speaking`] in its place when it is hidden while a
    /// reading is still under way (`reading`), for example after a dictation during a reading.
    #[must_use]
    pub fn or_speaking(self, reading: bool) -> Self {
        if reading && self == Self::Hidden {
            Self::Speaking
        } else {
            self
        }
    }

    /// The failure on screen, if this is an error.
    #[must_use]
    pub const fn error(&self) -> Option<&OverlayError> {
        match self {
            Self::Error(error) => Some(error),
            _ => None,
        }
    }
}

impl OverlayEvent {
    /// Whether a reading is under way after this event, given whether one was before
    /// (`was`).
    #[must_use]
    pub const fn reading(&self, was: bool) -> bool {
        match self {
            Self::SpeakingStarted => true,
            Self::SpeakingEnded => false,
            _ => was,
        }
    }
}

impl Stage {
    /// The stage's name for the indicator window.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Armed => "armed",
            Self::Listening => "listening",
            Self::Thinking => "thinking",
        }
    }
}

impl Fix {
    /// The fix's name for the indicator window.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::SoundSettings => "sound_settings",
            Self::Settings => "settings",
        }
    }
}

/// How lit the mark is, from 0 to 1, for a block of input with root-mean-square `rms`
/// (full scale is 1). Quieter than [`QUIET_DB`] is 0, louder than [`LOUD_DB`] is 1, and the
/// range between is linear in decibels.
#[must_use]
pub fn level_from_rms(rms: f32) -> f32 {
    let db = 20.0 * rms.log10();
    let level = ((db - QUIET_DB) / (LOUD_DB - QUIET_DB)).clamp(0.0, 1.0);
    if level.is_nan() { 0.0 } else { level }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_mic() -> OverlayError {
        OverlayError {
            reason: "Microphone not found.".into(),
            fix: Some(Fix::SoundSettings),
        }
    }

    fn run(events: impl IntoIterator<Item = OverlayEvent>) -> Vec<OverlayState> {
        let mut state = OverlayState::Hidden;
        events
            .into_iter()
            .map(|event| {
                state = state.after(event);
                state.clone()
            })
            .collect()
    }

    #[test]
    fn a_dictation_goes_armed_listening_thinking_inserted_then_hidden() {
        assert_eq!(
            run([
                OverlayEvent::KeyDown,
                OverlayEvent::Recording,
                OverlayEvent::KeyUp,
                OverlayEvent::Inserted,
                OverlayEvent::Elapsed,
            ]),
            [
                OverlayState::Dictating(Stage::Armed),
                OverlayState::Dictating(Stage::Listening),
                OverlayState::Dictating(Stage::Thinking),
                OverlayState::Inserted,
                OverlayState::Hidden,
            ]
        );
    }

    #[test]
    fn key_repeats_while_listening_or_thinking_change_nothing() {
        assert_eq!(
            OverlayState::Dictating(Stage::Listening).after(OverlayEvent::KeyDown),
            OverlayState::Dictating(Stage::Listening)
        );
        assert_eq!(
            OverlayState::Dictating(Stage::Thinking).after(OverlayEvent::KeyDown),
            OverlayState::Dictating(Stage::Thinking)
        );
    }

    #[test]
    fn a_new_press_after_a_result_arms_again() {
        for state in [
            OverlayState::Hidden,
            OverlayState::Inserted,
            OverlayState::Speaking,
            OverlayState::Error(no_mic()),
        ] {
            assert_eq!(
                state.after(OverlayEvent::KeyDown),
                OverlayState::Dictating(Stage::Armed),
                "from {state:?}"
            );
        }
    }

    #[test]
    fn a_quick_release_before_the_microphone_opens_still_thinks() {
        assert_eq!(
            OverlayState::Dictating(Stage::Armed).after(OverlayEvent::KeyUp),
            OverlayState::Dictating(Stage::Thinking)
        );
    }

    #[test]
    fn recording_only_matters_while_armed() {
        assert_eq!(
            OverlayState::Hidden.after(OverlayEvent::Recording),
            OverlayState::Hidden
        );
        assert_eq!(
            OverlayState::Dictating(Stage::Thinking).after(OverlayEvent::Recording),
            OverlayState::Dictating(Stage::Thinking)
        );
    }

    #[test]
    fn key_up_without_a_dictation_changes_nothing() {
        assert_eq!(
            OverlayState::Hidden.after(OverlayEvent::KeyUp),
            OverlayState::Hidden
        );
        assert_eq!(
            OverlayState::Inserted.after(OverlayEvent::KeyUp),
            OverlayState::Inserted
        );
    }

    #[test]
    fn inserted_only_follows_thinking() {
        assert_eq!(
            OverlayState::Hidden.after(OverlayEvent::Inserted),
            OverlayState::Hidden
        );
        assert_eq!(
            OverlayState::Dictating(Stage::Listening).after(OverlayEvent::Inserted),
            OverlayState::Dictating(Stage::Listening)
        );
    }

    #[test]
    fn a_discarded_dictation_hides_the_mark() {
        for state in [
            OverlayState::Dictating(Stage::Armed),
            OverlayState::Dictating(Stage::Listening),
            OverlayState::Dictating(Stage::Thinking),
        ] {
            assert_eq!(
                state.after(OverlayEvent::Discarded),
                OverlayState::Hidden,
                "from {state:?}"
            );
        }
        assert_eq!(
            OverlayState::Speaking.after(OverlayEvent::Discarded),
            OverlayState::Speaking
        );
    }

    #[test]
    fn failures_show_from_any_state() {
        for state in [
            OverlayState::Hidden,
            OverlayState::Dictating(Stage::Armed),
            OverlayState::Dictating(Stage::Thinking),
            OverlayState::Speaking,
        ] {
            assert_eq!(
                state.after(OverlayEvent::Failed(no_mic())),
                OverlayState::Error(no_mic())
            );
        }
    }

    #[test]
    fn an_error_leaves_when_clicked_or_after_its_time() {
        let error = OverlayState::Error(no_mic());
        assert_eq!(error.after(OverlayEvent::Dismissed), OverlayState::Hidden);
        assert_eq!(error.after(OverlayEvent::Elapsed), OverlayState::Hidden);
        assert_eq!(
            OverlayState::Speaking.after(OverlayEvent::Dismissed),
            OverlayState::Speaking
        );
    }

    #[test]
    fn elapsed_only_ends_lingering_states() {
        for state in [
            OverlayState::Dictating(Stage::Armed),
            OverlayState::Dictating(Stage::Listening),
            OverlayState::Dictating(Stage::Thinking),
            OverlayState::Speaking,
        ] {
            assert_eq!(state.after(OverlayEvent::Elapsed), state);
        }
    }

    #[test]
    fn reading_aloud_shows_speaking_until_it_ends() {
        assert_eq!(
            run([OverlayEvent::SpeakingStarted, OverlayEvent::SpeakingEnded]),
            [OverlayState::Speaking, OverlayState::Hidden]
        );
        assert_eq!(
            OverlayState::Inserted.after(OverlayEvent::SpeakingStarted),
            OverlayState::Speaking
        );
        assert_eq!(
            OverlayState::Hidden.after(OverlayEvent::SpeakingEnded),
            OverlayState::Hidden
        );
    }

    #[test]
    fn a_dictation_in_progress_wins_over_reading() {
        for state in [
            OverlayState::Dictating(Stage::Armed),
            OverlayState::Dictating(Stage::Listening),
            OverlayState::Dictating(Stage::Thinking),
        ] {
            assert_eq!(state.after(OverlayEvent::SpeakingStarted), state);
            assert_eq!(state.after(OverlayEvent::SpeakingEnded), state);
        }
    }

    #[test]
    fn inserted_and_errors_linger() {
        assert_eq!(
            OverlayState::Inserted.linger(),
            Some(Duration::from_millis(700))
        );
        assert_eq!(
            OverlayState::Error(no_mic()).linger(),
            Some(Duration::from_secs(6))
        );
    }

    #[test]
    fn other_states_stay_until_something_happens() {
        for state in [
            OverlayState::Hidden,
            OverlayState::Dictating(Stage::Armed),
            OverlayState::Dictating(Stage::Listening),
            OverlayState::Dictating(Stage::Thinking),
            OverlayState::Speaking,
        ] {
            assert_eq!(state.linger(), None, "{state:?}");
        }
    }

    #[test]
    fn only_hidden_is_invisible_and_only_errors_take_clicks() {
        assert!(!OverlayState::Hidden.is_visible(), "hidden");
        assert!(OverlayState::Dictating(Stage::Armed).is_visible(), "armed");
        assert!(OverlayState::Error(no_mic()).takes_clicks(), "error");
        assert!(
            !OverlayState::Dictating(Stage::Listening).takes_clicks(),
            "listening"
        );
        assert!(!OverlayState::Hidden.takes_clicks(), "hidden");
    }

    fn assert_level(rms: f32, expected: f32) {
        let level = level_from_rms(rms);
        assert!(
            (level - expected).abs() < 1e-4,
            "rms {rms} gave {level}, expected {expected}"
        );
    }

    #[test]
    fn level_is_linear_in_decibels_between_quiet_and_loud() {
        assert_level(0.001, 0.0);
        assert_level(0.003_162_3, 0.0);
        assert_level(0.01, 0.25);
        assert_level(0.031_623, 0.5);
        assert_level(0.316_23, 1.0);
        assert_level(1.0, 1.0);
    }

    #[test]
    fn silence_and_garbage_are_dark() {
        assert_level(0.0, 0.0);
        assert_level(-0.5, 0.0);
        assert_level(f32::NAN, 0.0);
    }

    #[test]
    fn every_state_has_a_name_for_the_window() {
        assert_eq!(
            [
                OverlayState::Hidden,
                OverlayState::Dictating(Stage::Armed),
                OverlayState::Dictating(Stage::Listening),
                OverlayState::Dictating(Stage::Thinking),
                OverlayState::Inserted,
                OverlayState::Speaking,
                OverlayState::Error(no_mic()),
            ]
            .map(|state| state.id()),
            [
                "hidden",
                "armed",
                "listening",
                "thinking",
                "inserted",
                "speaking",
                "error"
            ]
        );
        assert_eq!(
            [Fix::SoundSettings, Fix::Settings].map(Fix::id),
            ["sound_settings", "settings"]
        );
    }

    #[test]
    fn a_reading_shows_again_when_what_covered_it_hides() {
        assert_eq!(
            [
                OverlayState::Hidden.or_speaking(true),
                OverlayState::Hidden.or_speaking(false),
                OverlayState::Inserted.or_speaking(true),
            ],
            [
                OverlayState::Speaking,
                OverlayState::Hidden,
                OverlayState::Inserted
            ]
        );
        assert_eq!(
            [
                OverlayEvent::SpeakingStarted.reading(false),
                OverlayEvent::SpeakingEnded.reading(true),
                OverlayEvent::KeyDown.reading(true),
                OverlayEvent::KeyDown.reading(false),
            ],
            [true, false, true, false]
        );
    }

    #[test]
    fn only_an_error_has_a_failure() {
        assert_eq!(OverlayState::Error(no_mic()).error(), Some(&no_mic()));
        assert_eq!(OverlayState::Speaking.error(), None);
    }
}
