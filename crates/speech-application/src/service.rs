use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use kernel::Mood;
use platform_ports::{SelectionError, SelectionReader};
use speech_domain::{AudioClip, Prosody, ReadingText, Sentence, TextError, Voice, VoiceChoice};
use thiserror::Error;

use crate::CancelFlag;
use crate::ports::{AudioOutput, PlaybackError, SpeechSynthesizer, SynthesisError};

/// Characters of the selection shown back to the user before they pick a mood.
pub const PREVIEW_CHARS: usize = 160;

/// Why a read-aloud use case failed. Messages are written for the person using the app.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReadAloudError {
    /// The focused app had no selection.
    #[error("Select some text first, then press the read-aloud shortcut.")]
    NothingSelected,
    /// The selection could not be read.
    #[error(transparent)]
    Selection(SelectionError),
    /// The selection is empty after trimming, or too long.
    #[error(transparent)]
    InvalidText(#[from] TextError),
    /// A mood was chosen before any text was captured.
    #[error("There is no captured text yet. Select text and press the read-aloud shortcut.")]
    NothingCaptured,
    /// The voice could not produce audio.
    #[error(transparent)]
    Synthesis(#[from] SynthesisError),
    /// The speakers could not play the audio.
    #[error(transparent)]
    Playback(#[from] PlaybackError),
}

impl From<SelectionError> for ReadAloudError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::NothingSelected => Self::NothingSelected,
            SelectionError::Unavailable(_) => Self::Selection(error),
        }
    }
}

/// What the user sees of the captured selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPreview {
    /// The first [`PREVIEW_CHARS`] characters, whitespace collapsed.
    pub text: String,
    /// Length of the whole selection in characters.
    pub char_count: usize,
    /// The whole selection, split the way the voice reads it.
    pub sentences: Vec<Sentence>,
}

/// When a sentence just queued on the speakers will be heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceTiming {
    /// Position of the sentence in the captured text, from 0.
    pub index: usize,
    /// How long until it starts playing: the audio queued ahead of it.
    pub starts_in: Duration,
    /// How long its speech lasts, without the pause the mood adds after it.
    pub speech: Duration,
}

/// How a reading ended without an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// Every sentence was played.
    Finished,
    /// [`ReadAloudService::stop`] or a newer reading interrupted it.
    Stopped,
}

/// Read selected text aloud: capture the selection, then read it in a chosen mood.
///
/// Safe to share between threads. Starting a reading stops the previous one.
pub struct ReadAloudService {
    selection: Arc<dyn SelectionReader>,
    synthesizer: Arc<dyn SpeechSynthesizer>,
    output: Arc<dyn AudioOutput>,
    captured: Mutex<Option<ReadingText>>,
    current: Mutex<Session>,
}

impl fmt::Debug for ReadAloudService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadAloudService").finish_non_exhaustive()
    }
}

impl ReadAloudService {
    /// A service backed by the given adapters.
    #[must_use]
    pub fn new(
        selection: Arc<dyn SelectionReader>,
        synthesizer: Arc<dyn SpeechSynthesizer>,
        output: Arc<dyn AudioOutput>,
    ) -> Self {
        Self {
            selection,
            synthesizer,
            output,
            captured: Mutex::new(None),
            current: Mutex::new(Session::default()),
        }
    }

    /// Reads the current selection and keeps it for [`Self::read_captured`].
    ///
    /// # Errors
    ///
    /// [`ReadAloudError::NothingSelected`], [`ReadAloudError::Selection`], or
    /// [`ReadAloudError::InvalidText`]. On error the previously captured text is kept.
    pub fn capture_selection(&self) -> Result<SelectionPreview, ReadAloudError> {
        let raw = self.selection.read_selection()?;
        let text = ReadingText::parse(&raw)?;
        let preview = SelectionPreview {
            text: text.preview(PREVIEW_CHARS),
            char_count: text.char_count(),
            sentences: text.split(),
        };
        *lock(&self.captured) = Some(text);
        Ok(preview)
    }

    /// The voices the synthesizer offers.
    ///
    /// # Errors
    ///
    /// [`ReadAloudError::Synthesis`] when the voice server cannot be asked.
    pub fn voices(&self) -> Result<Vec<Voice>, ReadAloudError> {
        Ok(self.synthesizer.voices()?)
    }

    /// Reads the captured text in `mood` with `voice` (`None` is the synthesizer's default),
    /// sentence by sentence, and returns when it has finished playing or was stopped. Stops
    /// any reading already in progress first.
    ///
    /// # Errors
    ///
    /// [`ReadAloudError::NothingCaptured`], [`ReadAloudError::Synthesis`], or
    /// [`ReadAloudError::Playback`]. On error the speakers are silenced.
    pub fn read_captured(
        &self,
        mood: Mood,
        voice: Option<VoiceChoice>,
    ) -> Result<ReadOutcome, ReadAloudError> {
        let ticket = self.begin(mood, voice)?;
        self.finish(ticket, |_| ())
            .unwrap_or(Ok(ReadOutcome::Stopped))
    }

    /// Starts reading the captured text in `mood` with `voice` (`None` is the synthesizer's
    /// default), from the first sentence unless [`ReadingTicket::starting_at`] says otherwise:
    /// stops any reading in progress and makes this one current, so a [`Self::stop`] from now
    /// on ends it. Play it with [`Self::finish`], typically on another thread.
    ///
    /// # Errors
    ///
    /// [`ReadAloudError::NothingCaptured`].
    pub fn begin(
        &self,
        mood: Mood,
        voice: Option<VoiceChoice>,
    ) -> Result<ReadingTicket, ReadAloudError> {
        let text = lock(&self.captured)
            .clone()
            .ok_or(ReadAloudError::NothingCaptured)?;
        let session = self.restart();
        Ok(ReadingTicket {
            _active: ActiveSession(Arc::clone(&session.active)),
            reading: Reading {
                text,
                prosody: Prosody::for_mood(mood),
                voice,
                from: 0,
                cancel: session.cancel,
            },
        })
    }

    /// Plays a reading started with [`Self::begin`] until it ends or is stopped, calling
    /// `report` as each sentence is queued. A reading that was stopped or replaced reports
    /// nothing more. Returns `None` when a newer reading replaced it, because its outcome no
    /// longer matters to anyone; otherwise the outcome, with the speakers silenced on error.
    pub fn finish(
        &self,
        ticket: ReadingTicket,
        report: impl Fn(SentenceTiming),
    ) -> Option<Result<ReadOutcome, ReadAloudError>> {
        let ReadingTicket { reading, _active } = ticket;
        let outcome = self.speak(&reading, &report);
        self.settle(outcome, &reading.cancel)
    }

    /// Stops the current reading, if any, and silences the speakers.
    pub fn stop(&self) {
        let current = lock(&self.current);
        current.cancel.cancel();
        self.output.stop();
    }

    /// Whether the current reading is in progress and has not been stopped. Readings that
    /// a newer one replaced do not count, even while they are still winding down.
    #[must_use]
    pub fn is_reading(&self) -> bool {
        let current = lock(&self.current);
        current.active.load(Ordering::SeqCst) && !current.cancel.is_cancelled()
    }

    fn restart(&self) -> Session {
        let mut current = lock(&self.current);
        current.cancel.cancel();
        self.output.stop();
        *current = Session::started();
        current.clone()
    }

    fn settle(
        &self,
        outcome: Result<ReadOutcome, ReadAloudError>,
        cancel: &CancelFlag,
    ) -> Option<Result<ReadOutcome, ReadAloudError>> {
        let current = lock(&self.current);
        if !current.cancel.same(cancel) {
            return None;
        }
        if cancel.is_cancelled() {
            return Some(Ok(ReadOutcome::Stopped));
        }
        if outcome.is_err() {
            self.output.stop();
        }
        Some(outcome)
    }

    fn speak(
        &self,
        reading: &Reading,
        report: &dyn Fn(SentenceTiming),
    ) -> Result<ReadOutcome, ReadAloudError> {
        let cancel = &reading.cancel;
        let sentences = reading.text.sentences().into_iter().enumerate();
        let mut spoken = false;
        for (index, sentence) in sentences.skip(reading.from) {
            let step = Step {
                reading,
                index,
                sentence: &sentence,
                spoken,
            };
            match self.speak_one(&step, report)? {
                Voiced::Clip(()) => spoken = true,
                Voiced::Skip => {}
                Voiced::Stop => return Ok(ReadOutcome::Stopped),
            }
        }
        self.output.wait_until_idle(cancel);
        Ok(if cancel.is_cancelled() {
            ReadOutcome::Stopped
        } else {
            ReadOutcome::Finished
        })
    }

    /// Synthesizes and queues one sentence: `Clip` once it is queued, `Skip` when there is
    /// nothing to play, `Stop` when the reading was stopped.
    fn speak_one(
        &self,
        step: &Step<'_>,
        report: &dyn Fn(SentenceTiming),
    ) -> Result<Voiced<()>, ReadAloudError> {
        let clip = match self.voiced(step)? {
            Voiced::Clip(clip) => clip,
            Voiced::Skip => return Ok(Voiced::Skip),
            Voiced::Stop => return Ok(Voiced::Stop),
        };
        let queued = Queued {
            reading: step.reading,
            index: step.index,
            speech: step.reading.prosody.voice(&clip),
        };
        Ok(if self.enqueue_unless_cancelled(queued, report)? {
            Voiced::Clip(())
        } else {
            Voiced::Stop
        })
    }

    /// The sentence's audio, or why there is none. Sentences with nothing to say are not
    /// sent. A sentence the server refuses is skipped once the reading has been heard, so
    /// one odd line (a Markdown table rule) does not end it; before that the refusal is an
    /// error, since every sentence would likely fail the same way.
    fn voiced(&self, step: &Step<'_>) -> Result<Voiced<AudioClip>, SynthesisError> {
        if !ReadingText::is_speakable(step.sentence) {
            return Ok(Voiced::Skip);
        }
        match self.synthesize_unless_cancelled(step.sentence, step.reading) {
            Ok(Some(clip)) => Ok(Voiced::Clip(clip)),
            Ok(None) => Ok(Voiced::Stop),
            Err(SynthesisError::Server(_)) if step.spoken => Ok(Voiced::Skip),
            Err(error) => Err(error),
        }
    }

    fn synthesize_unless_cancelled(
        &self,
        sentence: &str,
        reading: &Reading,
    ) -> Result<Option<AudioClip>, SynthesisError> {
        if reading.cancel.is_cancelled() {
            return Ok(None);
        }
        reading
            .voice
            .as_ref()
            .map_or_else(
                || self.synthesizer.synthesize(sentence),
                |voice| self.synthesizer.synthesize_in(sentence, voice),
            )
            .map(Some)
    }

    /// Queues the sentence and reports when it will play, under the session lock so a
    /// reading stopped or replaced meanwhile neither plays nor reports it.
    fn enqueue_unless_cancelled(
        &self,
        queued: Queued<'_>,
        report: &dyn Fn(SentenceTiming),
    ) -> Result<bool, PlaybackError> {
        let Queued {
            reading,
            index,
            speech,
        } = queued;
        let _current = lock(&self.current);
        if reading.cancel.is_cancelled() {
            return Ok(false);
        }
        let length = speech.duration();
        let starts_in = self
            .output
            .enqueue(speech.with_trailing_silence(reading.prosody.pause_after()))?;
        report(SentenceTiming {
            index,
            starts_in,
            speech: length,
        });
        Ok(true)
    }
}

#[derive(Debug, Clone, Default)]
struct Session {
    cancel: CancelFlag,
    active: Arc<AtomicBool>,
}

impl Session {
    fn started() -> Self {
        Self {
            cancel: CancelFlag::new(),
            active: Arc::new(AtomicBool::new(true)),
        }
    }
}

/// A reading started with [`ReadAloudService::begin`] and not yet played.
pub struct ReadingTicket {
    reading: Reading,
    _active: ActiveSession,
}

impl ReadingTicket {
    /// The same reading, started from sentence `from` (0 is the first). Sentences before it
    /// are not synthesized.
    #[must_use]
    pub fn starting_at(mut self, from: usize) -> Self {
        self.reading.from = from;
        self
    }

    /// The same reading at `tempo_per_mille` (1000 = normal speed), pitch kept. See
    /// [`Prosody::at_tempo`] for the limits.
    #[must_use]
    pub fn at_tempo(mut self, tempo_per_mille: u32) -> Self {
        self.reading.prosody = self.reading.prosody.at_tempo(tempo_per_mille);
        self
    }
}

impl fmt::Debug for ReadingTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadingTicket").finish_non_exhaustive()
    }
}

struct Reading {
    text: ReadingText,
    prosody: Prosody,
    voice: Option<VoiceChoice>,
    from: usize,
    cancel: CancelFlag,
}

enum Voiced<T> {
    Clip(T),
    Skip,
    Stop,
}

struct Step<'a> {
    reading: &'a Reading,
    index: usize,
    sentence: &'a str,
    spoken: bool,
}

struct Queued<'a> {
    reading: &'a Reading,
    index: usize,
    speech: AudioClip,
}

struct ActiveSession(Arc<AtomicBool>);

impl Drop for ActiveSession {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    type Hook = Box<dyn FnOnce() + Send>;

    struct FakeSelection(Result<String, SelectionError>);

    impl SelectionReader for FakeSelection {
        fn read_selection(&self) -> Result<String, SelectionError> {
            self.0.clone()
        }
    }

    #[derive(Default)]
    struct FakeSynth {
        spoken: Mutex<Vec<String>>,
        fail_on: Option<&'static str>,
        unreachable_on: Option<&'static str>,
        after_first: Mutex<Option<Hook>>,
        chosen: Mutex<Vec<(String, VoiceChoice)>>,
        offered: Vec<Voice>,
        voices_fail: bool,
    }

    impl SpeechSynthesizer for FakeSynth {
        fn synthesize(&self, sentence: &str) -> Result<AudioClip, SynthesisError> {
            lock(&self.spoken).push(sentence.to_owned());
            let hook = lock(&self.after_first).take();
            if let Some(hook) = hook {
                hook();
            }
            if self.fail_on == Some(sentence) {
                return Err(SynthesisError::Server("voice missing".into()));
            }
            if self.unreachable_on == Some(sentence) {
                return Err(SynthesisError::Unreachable {
                    address: "voice-box:10200".into(),
                    reason: "connection reset".into(),
                });
            }
            Ok(AudioClip::from_samples(1_000, vec![0.1; 100]).unwrap())
        }

        fn synthesize_in(
            &self,
            sentence: &str,
            voice: &VoiceChoice,
        ) -> Result<AudioClip, SynthesisError> {
            lock(&self.chosen).push((sentence.to_owned(), voice.clone()));
            Ok(AudioClip::from_samples(1_000, vec![0.1; 100]).unwrap())
        }

        fn voices(&self) -> Result<Vec<Voice>, SynthesisError> {
            if self.voices_fail {
                return Err(SynthesisError::Unreachable {
                    address: "voice-box:10200".into(),
                    reason: "connection refused".into(),
                });
            }
            Ok(self.offered.clone())
        }
    }

    #[derive(Default)]
    struct FakeOutput {
        queued: Mutex<Vec<usize>>,
        ahead: Mutex<Duration>,
        stops: AtomicUsize,
        waits: AtomicUsize,
        broken: bool,
    }

    impl AudioOutput for FakeOutput {
        fn enqueue(&self, clip: AudioClip) -> Result<Duration, PlaybackError> {
            if self.broken {
                return Err(PlaybackError("no device".into()));
            }
            lock(&self.queued).push(clip.samples().len());
            let mut ahead = lock(&self.ahead);
            let starts_in = *ahead;
            *ahead += clip.duration();
            Ok(starts_in)
        }

        fn wait_until_idle(&self, _cancel: &CancelFlag) {
            self.waits.fetch_add(1, Ordering::SeqCst);
        }

        fn stop(&self) {
            self.stops.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Rig {
        service: Arc<ReadAloudService>,
        synth: Arc<FakeSynth>,
        output: Arc<FakeOutput>,
    }

    fn rig(selection: Result<&str, SelectionError>, synth: FakeSynth, output: FakeOutput) -> Rig {
        let synth = Arc::new(synth);
        let output = Arc::new(output);
        let synthesizer: Arc<dyn SpeechSynthesizer> = Arc::<FakeSynth>::clone(&synth);
        let speakers: Arc<dyn AudioOutput> = Arc::<FakeOutput>::clone(&output);
        let service = Arc::new(ReadAloudService::new(
            Arc::new(FakeSelection(selection.map(str::to_owned))),
            synthesizer,
            speakers,
        ));
        Rig {
            service,
            synth,
            output,
        }
    }

    fn reading(selection: &str) -> Rig {
        let rig = rig(Ok(selection), FakeSynth::default(), FakeOutput::default());
        rig.service.capture_selection().unwrap();
        rig
    }

    #[test]
    fn capture_returns_a_preview_the_length_and_the_sentences() {
        let rig = rig(
            Ok("  Hello\n world  "),
            FakeSynth::default(),
            FakeOutput::default(),
        );
        assert_eq!(
            rig.service.capture_selection(),
            Ok(SelectionPreview {
                text: "Hello world".into(),
                char_count: 12,
                sentences: vec![
                    Sentence {
                        text: "Hello".into(),
                        starts_paragraph: true
                    },
                    Sentence {
                        text: "world".into(),
                        starts_paragraph: true
                    },
                ],
            })
        );
    }

    #[test]
    fn capture_shortens_long_previews() {
        let long = "word ".repeat(100);
        let rig = rig(Ok(&long), FakeSynth::default(), FakeOutput::default());
        let preview = rig.service.capture_selection().unwrap();
        assert_eq!(preview.char_count, 499);
        assert!(
            preview.text.chars().count() <= PREVIEW_CHARS + 1,
            "preview is at most the limit plus an ellipsis"
        );
        assert!(
            preview.text.ends_with('\u{2026}'),
            "a cut preview ends with an ellipsis"
        );
    }

    #[test]
    fn capture_reports_missing_unreadable_and_blank_selections() {
        let nothing = rig(
            Err(SelectionError::NothingSelected),
            FakeSynth::default(),
            FakeOutput::default(),
        );
        assert_eq!(
            nothing.service.capture_selection(),
            Err(ReadAloudError::NothingSelected)
        );
        let busy = SelectionError::Unavailable("clipboard busy".into());
        let unreadable = rig(
            Err(busy.clone()),
            FakeSynth::default(),
            FakeOutput::default(),
        );
        assert_eq!(
            unreadable.service.capture_selection(),
            Err(ReadAloudError::Selection(busy))
        );
        let blank = rig(Ok(" \n "), FakeSynth::default(), FakeOutput::default());
        assert_eq!(
            blank.service.capture_selection(),
            Err(ReadAloudError::InvalidText(TextError::Empty))
        );
    }

    #[test]
    fn reading_before_any_capture_is_an_error() {
        let rig = rig(Ok("Hi."), FakeSynth::default(), FakeOutput::default());
        assert_eq!(
            rig.service.read_captured(Mood::Neutral, None),
            Err(ReadAloudError::NothingCaptured)
        );
    }

    #[test]
    fn reads_every_sentence_in_order_with_the_mood_applied() {
        let rig = reading("One. Two.");
        assert_eq!(
            rig.service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Finished)
        );
        assert_eq!(*lock(&rig.synth.spoken), ["One.", "Two."]);
        assert_eq!(*lock(&rig.output.queued), [100 + 250, 100 + 250]);
        assert_eq!(rig.output.waits.load(Ordering::SeqCst), 1);
        assert!(!rig.service.is_reading(), "not reading after it finished");
    }

    #[test]
    fn captured_text_can_be_read_again_in_another_mood() {
        let rig = reading("Hello.");
        rig.service.read_captured(Mood::Neutral, None).unwrap();
        rig.service.read_captured(Mood::Mad, None).unwrap();
        assert_eq!(*lock(&rig.output.queued), [100 + 250, 80 + 80]);
    }

    #[test]
    fn a_synthesis_failure_stops_the_reading_and_silences_the_speakers() {
        let rig = rig(
            Ok("One. Two. Three."),
            FakeSynth {
                fail_on: Some("One."),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        rig.service.capture_selection().unwrap();
        assert_eq!(
            rig.service.read_captured(Mood::Neutral, None),
            Err(ReadAloudError::Synthesis(SynthesisError::Server(
                "voice missing".into()
            )))
        );
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
        assert_eq!(
            rig.output.stops.load(Ordering::SeqCst),
            2,
            "silenced once when the reading started and once on the failure"
        );
    }

    #[test]
    fn a_sentence_the_server_refuses_after_the_first_is_skipped() {
        let rig = rig(
            Ok("One. Two. Three."),
            FakeSynth {
                fail_on: Some("Two."),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        rig.service.capture_selection().unwrap();
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        let (outcome, reports) = reported(&rig.service, ticket);
        assert_eq!(outcome, Some(Ok(ReadOutcome::Finished)));
        assert_eq!(*lock(&rig.synth.spoken), ["One.", "Two.", "Three."]);
        assert_eq!(
            reports
                .iter()
                .map(|timing| timing.index)
                .collect::<Vec<_>>(),
            [0, 2],
            "the refused sentence is not reported"
        );
    }

    #[test]
    fn losing_the_voice_server_mid_reading_still_fails() {
        let rig = rig(
            Ok("One. Two. Three."),
            FakeSynth {
                unreachable_on: Some("Two."),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        rig.service.capture_selection().unwrap();
        assert!(
            matches!(
                rig.service.read_captured(Mood::Neutral, None),
                Err(ReadAloudError::Synthesis(
                    SynthesisError::Unreachable { .. }
                ))
            ),
            "a network failure ends the reading"
        );
    }

    #[test]
    fn lines_with_nothing_to_say_are_not_sent_to_the_voice() {
        let rig = reading(
            "One.
| --- | --- |
Two.",
        );
        let (outcome, reports) = reported(
            &rig.service,
            rig.service.begin(Mood::Neutral, None).unwrap(),
        );
        assert_eq!(outcome, Some(Ok(ReadOutcome::Finished)));
        assert_eq!(*lock(&rig.synth.spoken), ["One.", "Two."]);
        assert_eq!(
            reports
                .iter()
                .map(|timing| timing.index)
                .collect::<Vec<_>>(),
            [0, 2]
        );
    }

    #[test]
    fn a_playback_failure_is_reported() {
        let rig = rig(
            Ok("One."),
            FakeSynth::default(),
            FakeOutput {
                broken: true,
                ..FakeOutput::default()
            },
        );
        rig.service.capture_selection().unwrap();
        assert_eq!(
            rig.service.read_captured(Mood::Neutral, None),
            Err(ReadAloudError::Playback(PlaybackError("no device".into())))
        );
    }

    #[test]
    fn stopping_mid_reading_skips_the_rest() {
        let rig = reading("One. Two. Three.");
        let service = Arc::clone(&rig.service);
        let was_reading = Arc::new(Mutex::new(None));
        let seen = Arc::clone(&was_reading);
        *lock(&rig.synth.after_first) = Some(Box::new(move || {
            *lock(&seen) = Some(service.is_reading());
            service.stop();
        }));
        assert_eq!(
            rig.service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Stopped)
        );
        assert_eq!(*lock(&was_reading), Some(true));
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
        assert!(
            lock(&rig.output.queued).is_empty(),
            "audio synthesized after the stop is never queued"
        );
        assert!(!rig.service.is_reading(), "not reading once stopped");
    }

    #[test]
    fn starting_a_reading_silences_the_previous_one() {
        let rig = reading("One.");
        rig.service.read_captured(Mood::Neutral, None).unwrap();
        assert_eq!(rig.output.stops.load(Ordering::SeqCst), 1);
    }

    struct FailFirst {
        hook: Mutex<Option<Hook>>,
        failed: AtomicBool,
    }

    impl FailFirst {
        fn new(hook: Hook) -> Self {
            Self {
                hook: Mutex::new(Some(hook)),
                failed: AtomicBool::new(false),
            }
        }
    }

    impl SpeechSynthesizer for FailFirst {
        fn synthesize(&self, _sentence: &str) -> Result<AudioClip, SynthesisError> {
            let first = !self.failed.swap(true, Ordering::SeqCst);
            let hook = lock(&self.hook).take();
            if let Some(hook) = hook {
                hook();
            }
            if first {
                return Err(SynthesisError::Server("voice missing".into()));
            }
            Ok(AudioClip::from_samples(1_000, vec![0.1; 100]).unwrap())
        }

        fn voices(&self) -> Result<Vec<Voice>, SynthesisError> {
            Ok(Vec::new())
        }
    }

    struct SucceedAfterHook {
        hook: Mutex<Option<Hook>>,
    }

    impl SpeechSynthesizer for SucceedAfterHook {
        fn synthesize(&self, _sentence: &str) -> Result<AudioClip, SynthesisError> {
            let hook = lock(&self.hook).take();
            if let Some(hook) = hook {
                hook();
            }
            Ok(AudioClip::from_samples(1_000, vec![0.1; 100]).unwrap())
        }

        fn voices(&self) -> Result<Vec<Voice>, SynthesisError> {
            Ok(Vec::new())
        }
    }

    fn service_with(
        synthesizer: Arc<dyn SpeechSynthesizer>,
        output: Arc<dyn AudioOutput>,
    ) -> Arc<ReadAloudService> {
        let service = Arc::new(ReadAloudService::new(
            Arc::new(FakeSelection(Ok("One.".into()))),
            synthesizer,
            output,
        ));
        service.capture_selection().unwrap();
        service
    }

    type Slot<T> = Arc<Mutex<Option<T>>>;

    fn hook_into(
        slot: &Slot<Arc<ReadAloudService>>,
        work: impl FnOnce(&ReadAloudService) + Send + 'static,
    ) -> Hook {
        let slot = Arc::clone(slot);
        Box::new(move || {
            let service = lock(&slot).clone().unwrap();
            work(&service);
        })
    }

    #[test]
    fn a_failure_after_the_reading_was_stopped_is_a_stop_not_an_error() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let synth = FailFirst::new(hook_into(&slot, ReadAloudService::stop));
        let output = Arc::new(FakeOutput::default());
        let service = service_with(Arc::new(synth), Arc::<FakeOutput>::clone(&output));
        *lock(&slot) = Some(Arc::clone(&service));
        assert_eq!(
            service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Stopped)
        );
        assert_eq!(
            output.stops.load(Ordering::SeqCst),
            2,
            "once when the reading started and once for the stop, none for the late failure"
        );
        assert!(!service.is_reading());
    }

    #[test]
    fn a_failure_from_a_replaced_reading_neither_errors_nor_silences_the_new_one() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let replacement: Slot<Result<ReadOutcome, ReadAloudError>> = Arc::default();
        let still_reading: Slot<bool> = Arc::default();
        let (outcome, seen) = (Arc::clone(&replacement), Arc::clone(&still_reading));
        let synth = FailFirst::new(hook_into(&slot, move |service| {
            *lock(&outcome) = Some(service.read_captured(Mood::Neutral, None));
            *lock(&seen) = Some(service.is_reading());
        }));
        let output = Arc::new(FakeOutput::default());
        let service = service_with(Arc::new(synth), Arc::<FakeOutput>::clone(&output));
        *lock(&slot) = Some(Arc::clone(&service));
        assert_eq!(
            service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Stopped)
        );
        assert_eq!(*lock(&replacement), Some(Ok(ReadOutcome::Finished)));
        assert_eq!(
            *lock(&still_reading),
            Some(false),
            "the replaced reading, still blocked in synthesis, is not the current one"
        );
        assert_eq!(
            output.stops.load(Ordering::SeqCst),
            2,
            "one stop per reading start, none for the replaced reading's failure"
        );
        assert_eq!(*lock(&output.queued), [100 + 250]);
    }

    #[test]
    fn a_failure_from_the_current_reading_is_reported_and_silences_the_speakers() {
        let synth = FailFirst::new(Box::new(|| {}));
        let output = Arc::new(FakeOutput::default());
        let service = service_with(Arc::new(synth), Arc::<FakeOutput>::clone(&output));
        assert_eq!(
            service.read_captured(Mood::Neutral, None),
            Err(ReadAloudError::Synthesis(SynthesisError::Server(
                "voice missing".into()
            )))
        );
        assert_eq!(output.stops.load(Ordering::SeqCst), 2);
        assert!(!service.is_reading());
    }

    #[test]
    fn a_reading_stopped_between_synthesis_and_playback_queues_nothing() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let synth = SucceedAfterHook {
            hook: Mutex::new(Some(hook_into(&slot, ReadAloudService::stop))),
        };
        let output = Arc::new(FakeOutput::default());
        let service = service_with(Arc::new(synth), Arc::<FakeOutput>::clone(&output));
        *lock(&slot) = Some(Arc::clone(&service));
        assert_eq!(
            service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Stopped)
        );
        assert!(lock(&output.queued).is_empty());
        assert_eq!(output.waits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_replaced_reading_never_queues_its_audio() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let synth = SucceedAfterHook {
            hook: Mutex::new(Some(hook_into(&slot, |service| {
                service.read_captured(Mood::Neutral, None).unwrap();
            }))),
        };
        let output = Arc::new(FakeOutput::default());
        let service = service_with(Arc::new(synth), Arc::<FakeOutput>::clone(&output));
        *lock(&slot) = Some(Arc::clone(&service));
        assert_eq!(
            service.read_captured(Mood::Neutral, None),
            Ok(ReadOutcome::Stopped)
        );
        assert_eq!(
            *lock(&output.queued),
            [100 + 250],
            "only the replacement's audio"
        );
    }

    #[test]
    fn is_reading_is_true_only_while_the_current_reading_runs() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let during: Slot<bool> = Arc::default();
        let seen = Arc::clone(&during);
        let synth = SucceedAfterHook {
            hook: Mutex::new(Some(hook_into(&slot, move |service| {
                *lock(&seen) = Some(service.is_reading());
            }))),
        };
        let service = service_with(Arc::new(synth), Arc::new(FakeOutput::default()));
        *lock(&slot) = Some(Arc::clone(&service));
        assert!(!service.is_reading(), "idle before");
        service.read_captured(Mood::Neutral, None).unwrap();
        assert_eq!(*lock(&during), Some(true));
        assert!(!service.is_reading(), "idle after");
    }

    #[test]
    fn beginning_without_captured_text_is_an_error() {
        let rig = rig(Ok("Hi."), FakeSynth::default(), FakeOutput::default());
        assert!(matches!(
            rig.service.begin(Mood::Neutral, None),
            Err(ReadAloudError::NothingCaptured)
        ));
    }

    #[test]
    fn a_begun_reading_is_current_until_it_is_finished() {
        let rig = reading("One.");
        assert!(!rig.service.is_reading());
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        assert!(rig.service.is_reading(), "current as soon as it begins");
        assert_eq!(
            rig.service.finish(ticket, |_| ()),
            Some(Ok(ReadOutcome::Finished))
        );
        assert!(!rig.service.is_reading());
    }

    #[test]
    fn beginning_silences_the_speakers_but_speaks_nothing_until_finished() {
        let rig = reading("One.");
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        assert_eq!(rig.output.stops.load(Ordering::SeqCst), 1);
        assert!(lock(&rig.synth.spoken).is_empty(), "nothing is spoken yet");
        rig.service.finish(ticket, |_| ());
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
    }

    #[test]
    fn dropping_a_ticket_unfinished_ends_the_reading() {
        let rig = reading("One.");
        drop(rig.service.begin(Mood::Neutral, None).unwrap());
        assert!(!rig.service.is_reading());
    }

    #[test]
    fn a_stop_between_begin_and_finish_finishes_stopped_without_speaking() {
        let rig = reading("One. Two.");
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        rig.service.stop();
        assert!(!rig.service.is_reading());
        assert_eq!(
            rig.service.finish(ticket, |_| ()),
            Some(Ok(ReadOutcome::Stopped))
        );
        assert!(lock(&rig.synth.spoken).is_empty());
        assert!(lock(&rig.output.queued).is_empty());
    }

    #[test]
    fn a_replaced_ticket_finishes_as_none_without_speaking() {
        let rig = reading("One.");
        let old = rig.service.begin(Mood::Neutral, None).unwrap();
        let new = rig.service.begin(Mood::Neutral, None).unwrap();
        assert_eq!(rig.service.finish(old, |_| ()), None);
        assert!(lock(&rig.synth.spoken).is_empty());
        assert!(
            rig.service.is_reading(),
            "the newer reading is still current"
        );
        assert_eq!(
            rig.service.finish(new, |_| ()),
            Some(Ok(ReadOutcome::Finished))
        );
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
    }

    #[test]
    fn a_replaced_ticket_that_fails_is_none_and_leaves_the_speakers_alone() {
        let rig = rig(
            Ok("One."),
            FakeSynth {
                fail_on: Some("One."),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        rig.service.capture_selection().unwrap();
        let old = rig.service.begin(Mood::Neutral, None).unwrap();
        let _new = rig.service.begin(Mood::Neutral, None).unwrap();
        let stops_before = rig.output.stops.load(Ordering::SeqCst);
        assert_eq!(rig.service.finish(old, |_| ()), None);
        assert_eq!(rig.output.stops.load(Ordering::SeqCst), stops_before);
    }

    #[test]
    fn a_current_ticket_that_fails_returns_the_error_and_silences_the_speakers() {
        let rig = rig(
            Ok("One."),
            FakeSynth {
                fail_on: Some("One."),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        rig.service.capture_selection().unwrap();
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        assert_eq!(
            rig.service.finish(ticket, |_| ()),
            Some(Err(ReadAloudError::Synthesis(SynthesisError::Server(
                "voice missing".into()
            ))))
        );
        assert_eq!(rig.output.stops.load(Ordering::SeqCst), 2);
    }

    fn reported(
        service: &ReadAloudService,
        ticket: ReadingTicket,
    ) -> (
        Option<Result<ReadOutcome, ReadAloudError>>,
        Vec<SentenceTiming>,
    ) {
        let reports = Mutex::new(Vec::new());
        let outcome = service.finish(ticket, |timing| lock(&reports).push(timing));
        (outcome, reports.into_inner().unwrap())
    }

    #[test]
    fn a_reading_from_a_later_sentence_skips_the_ones_before_it() {
        let rig = reading("One. Two. Three.");
        let ticket = rig
            .service
            .begin(Mood::Neutral, None)
            .unwrap()
            .starting_at(1);
        let (outcome, reports) = reported(&rig.service, ticket);
        assert_eq!(outcome, Some(Ok(ReadOutcome::Finished)));
        assert_eq!(*lock(&rig.synth.spoken), ["Two.", "Three."]);
        assert_eq!(
            reports
                .iter()
                .map(|timing| timing.index)
                .collect::<Vec<_>>(),
            [1, 2]
        );
    }

    #[test]
    fn a_reading_from_past_the_end_reads_nothing_and_finishes() {
        let rig = reading("One.");
        let ticket = rig
            .service
            .begin(Mood::Neutral, None)
            .unwrap()
            .starting_at(1);
        let (outcome, reports) = reported(&rig.service, ticket);
        assert_eq!(outcome, Some(Ok(ReadOutcome::Finished)));
        assert!(lock(&rig.synth.spoken).is_empty());
        assert_eq!(reports, []);
    }

    #[test]
    fn each_report_says_when_its_sentence_starts_and_how_long_its_speech_lasts() {
        let rig = reading("One. Two.");
        *lock(&rig.output.ahead) = Duration::from_secs(2);
        let ticket = rig.service.begin(Mood::Calm, None).unwrap();
        let (_, reports) = reported(&rig.service, ticket);
        let calm_speech = Duration::from_millis(108);
        let calm_pause = Duration::from_millis(420);
        assert_eq!(
            reports,
            [
                SentenceTiming {
                    index: 0,
                    starts_in: Duration::from_secs(2),
                    speech: calm_speech,
                },
                SentenceTiming {
                    index: 1,
                    starts_in: Duration::from_secs(2) + calm_speech + calm_pause,
                    speech: calm_speech,
                },
            ]
        );
    }

    #[test]
    fn a_faster_reading_shortens_each_sentence_and_its_pause() {
        let rig = reading("One. Two.");
        let ticket = rig
            .service
            .begin(Mood::Neutral, None)
            .unwrap()
            .at_tempo(2_000);
        let (_, reports) = reported(&rig.service, ticket);
        let speech = Duration::from_millis(50);
        assert_eq!(
            reports
                .iter()
                .map(|timing| (timing.starts_in, timing.speech))
                .collect::<Vec<_>>(),
            [
                (Duration::ZERO, speech),
                (speech + Duration::from_millis(125), speech),
            ]
        );
    }

    #[test]
    fn a_replaced_reading_reports_nothing() {
        let slot: Slot<Arc<ReadAloudService>> = Arc::default();
        let newer: Slot<Vec<SentenceTiming>> = Arc::default();
        let seen = Arc::clone(&newer);
        let synth = SucceedAfterHook {
            hook: Mutex::new(Some(hook_into(&slot, move |service| {
                let ticket = service.begin(Mood::Neutral, None).unwrap();
                *lock(&seen) = Some(reported(service, ticket).1);
            }))),
        };
        let service = service_with(Arc::new(synth), Arc::new(FakeOutput::default()));
        *lock(&slot) = Some(Arc::clone(&service));
        let ticket = service.begin(Mood::Neutral, None).unwrap();
        let (outcome, reports) = reported(&service, ticket);
        assert_eq!(outcome, None);
        assert_eq!(reports, [], "the replaced reading reports nothing");
        assert_eq!(
            lock(&newer).as_ref().map(Vec::len),
            Some(1),
            "the newer reading still reports"
        );
    }

    #[test]
    fn a_ticket_has_a_debug_form_that_hides_the_text() {
        let rig = reading("Secret.");
        let ticket = rig.service.begin(Mood::Neutral, None).unwrap();
        assert_eq!(format!("{ticket:?}"), "ReadingTicket { .. }");
    }

    struct LockProbe {
        service: Mutex<Option<Arc<ReadAloudService>>>,
        observations: Mutex<Vec<(&'static str, bool)>>,
    }

    impl LockProbe {
        fn observe(&self, what: &'static str) {
            let service = lock(&self.service).clone();
            if let Some(service) = service {
                let held = service.current.try_lock().is_err();
                lock(&self.observations).push((what, held));
            }
        }
    }

    impl AudioOutput for LockProbe {
        fn enqueue(&self, _clip: AudioClip) -> Result<Duration, PlaybackError> {
            self.observe("enqueue");
            Ok(Duration::ZERO)
        }

        fn wait_until_idle(&self, _cancel: &CancelFlag) {}

        fn stop(&self) {
            self.observe("stop");
        }
    }

    #[test]
    fn the_session_lock_is_held_while_queuing_stopping_and_settling() {
        let probe = Arc::new(LockProbe {
            service: Mutex::new(None),
            observations: Mutex::new(Vec::new()),
        });
        let service = service_with(
            Arc::new(FakeSynth::default()),
            Arc::<LockProbe>::clone(&probe),
        );
        *lock(&probe.service) = Some(Arc::clone(&service));
        service.read_captured(Mood::Neutral, None).unwrap();
        service.stop();
        let failing = service_with(
            Arc::new(FakeSynth {
                fail_on: Some("One."),
                ..FakeSynth::default()
            }),
            Arc::<LockProbe>::clone(&probe),
        );
        *lock(&probe.service) = Some(Arc::clone(&failing));
        assert!(failing.read_captured(Mood::Neutral, None).is_err());
        let seen = lock(&probe.observations).clone();
        assert!(seen.iter().any(|(what, _)| *what == "enqueue"));
        assert!(
            seen.iter().all(|(_, held)| *held),
            "every enqueue and stop happens under the lock: {seen:?}"
        );
        assert_eq!(seen.iter().filter(|(what, _)| *what == "stop").count(), 4);
    }

    fn choice(name: &str, speaker: Option<&str>) -> VoiceChoice {
        VoiceChoice::parse(Some(name), speaker).unwrap()
    }

    #[test]
    fn without_a_voice_the_default_synthesis_is_used() {
        let rig = reading("One. Two.");
        rig.service.read_captured(Mood::Neutral, None).unwrap();
        assert_eq!(*lock(&rig.synth.spoken), ["One.", "Two."]);
        assert!(lock(&rig.synth.chosen).is_empty(), "no voice was chosen");
    }

    #[test]
    fn a_chosen_voice_is_used_for_every_sentence() {
        let rig = reading("One. Two.");
        rig.service
            .read_captured(
                Mood::Neutral,
                Some(choice("en_US-libritts-high", Some("p239"))),
            )
            .unwrap();
        assert_eq!(
            *lock(&rig.synth.chosen),
            [
                (
                    "One.".to_owned(),
                    choice("en_US-libritts-high", Some("p239"))
                ),
                (
                    "Two.".to_owned(),
                    choice("en_US-libritts-high", Some("p239"))
                ),
            ]
        );
        assert!(
            lock(&rig.synth.spoken).is_empty(),
            "the default synthesis is not used"
        );
        assert_eq!(*lock(&rig.output.queued), [100 + 250, 100 + 250]);
    }

    #[test]
    fn a_ticket_uses_the_voice_it_began_with() {
        let rig = reading("One.");
        let ticket = rig
            .service
            .begin(Mood::Neutral, Some(choice("amy", None)))
            .unwrap();
        assert_eq!(
            rig.service.finish(ticket, |_| ()),
            Some(Ok(ReadOutcome::Finished))
        );
        assert_eq!(
            *lock(&rig.synth.chosen),
            [("One.".to_owned(), choice("amy", None))]
        );
        assert!(lock(&rig.synth.spoken).is_empty());
    }

    #[test]
    fn each_reading_keeps_its_own_voice_when_a_newer_one_began_first() {
        let rig = reading("One.");
        let old = rig
            .service
            .begin(Mood::Neutral, Some(choice("amy", None)))
            .unwrap();
        let new = rig
            .service
            .begin(Mood::Neutral, Some(choice("thorsten", None)))
            .unwrap();
        assert_eq!(rig.service.finish(old, |_| ()), None);
        assert!(
            lock(&rig.synth.chosen).is_empty(),
            "the replaced reading was cut short before synthesizing"
        );
        assert_eq!(
            rig.service.finish(new, |_| ()),
            Some(Ok(ReadOutcome::Finished))
        );
        assert_eq!(
            *lock(&rig.synth.chosen),
            [("One.".to_owned(), choice("thorsten", None))]
        );
        assert!(lock(&rig.synth.spoken).is_empty());
    }

    #[test]
    fn a_newer_reading_without_a_voice_does_not_inherit_the_older_ones() {
        let rig = reading("One.");
        let old = rig
            .service
            .begin(Mood::Neutral, Some(choice("amy", None)))
            .unwrap();
        let new = rig.service.begin(Mood::Neutral, None).unwrap();
        assert_eq!(
            rig.service.finish(new, |_| ()),
            Some(Ok(ReadOutcome::Finished))
        );
        assert_eq!(rig.service.finish(old, |_| ()), None);
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
        assert!(lock(&rig.synth.chosen).is_empty());
    }

    #[test]
    fn a_reading_does_not_remember_the_voice_of_the_one_before() {
        let rig = reading("One.");
        rig.service
            .read_captured(Mood::Neutral, Some(choice("amy", None)))
            .unwrap();
        rig.service.read_captured(Mood::Neutral, None).unwrap();
        assert_eq!(
            *lock(&rig.synth.chosen),
            [("One.".to_owned(), choice("amy", None))]
        );
        assert_eq!(*lock(&rig.synth.spoken), ["One."]);
    }

    #[test]
    fn the_voices_are_the_ones_the_synthesizer_offers() {
        let offered = vec![Voice {
            name: "en_US-amy-medium".into(),
            description: "amy (medium)".into(),
            languages: vec!["en_US".into()],
            speakers: Vec::new(),
        }];
        let rig = rig(
            Ok("One."),
            FakeSynth {
                offered: offered.clone(),
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        assert_eq!(rig.service.voices(), Ok(offered));
    }

    #[test]
    fn a_voice_server_failure_is_a_synthesis_error() {
        let rig = rig(
            Ok("One."),
            FakeSynth {
                voices_fail: true,
                ..FakeSynth::default()
            },
            FakeOutput::default(),
        );
        assert_eq!(
            rig.service.voices(),
            Err(ReadAloudError::Synthesis(SynthesisError::Unreachable {
                address: "voice-box:10200".into(),
                reason: "connection refused".into(),
            }))
        );
    }

    #[test]
    fn error_messages_are_written_for_people() {
        assert_eq!(
            ReadAloudError::NothingSelected.to_string(),
            "Select some text first, then press the read-aloud shortcut."
        );
        assert_eq!(
            ReadAloudError::NothingCaptured.to_string(),
            "There is no captured text yet. Select text and press the read-aloud shortcut."
        );
        assert_eq!(
            ReadAloudError::from(SelectionError::Unavailable("busy".into())).to_string(),
            "could not read the selection: busy"
        );
    }

    #[test]
    fn debug_does_not_leak_adapters() {
        let rig = reading("Hi.");
        assert_eq!(format!("{:?}", rig.service), "ReadAloudService { .. }");
    }
}
