use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use platform_ports::{InsertError, TextInserter};
use speech_domain::{AudioClip, level_from_rms};
use thiserror::Error;

use crate::ports::{AudioInput, CaptureError, SpeechToText, TranscriptionError};

/// Recordings shorter than this are taps on the key, not speech, and are never transcribed.
pub const MIN_SPEECH: Duration = Duration::from_millis(250);

/// Why a dictation failed. Messages are written for the person using the app.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DictationError {
    /// The microphone could not record.
    #[error(transparent)]
    Capture(#[from] CaptureError),
    /// The recording could not be turned into text.
    #[error(transparent)]
    Transcription(#[from] TranscriptionError),
    /// The text could not be put into the focused app.
    #[error(transparent)]
    Insertion(#[from] InsertError),
}

/// What pressing the dictation key did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressOutcome {
    /// The microphone is now recording.
    Listening,
    /// It was already recording (the key repeats while held).
    AlreadyListening,
    /// The previous dictation is still being transcribed or inserted, so this press is ignored.
    Busy,
}

/// How a dictation ended without an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationOutcome {
    /// The recording was shorter than [`MIN_SPEECH`], so nothing was transcribed.
    TooShort,
    /// The recording held no words.
    NothingHeard,
    /// The words were typed into the focused app, followed by a space.
    Inserted {
        /// Characters of text inserted, without the trailing space.
        chars: usize,
    },
}

impl fmt::Display for PressOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Listening => "listening",
            Self::AlreadyListening => "already listening",
            Self::Busy => "still typing the last dictation, so this press is ignored",
        })
    }
}

impl fmt::Display for DictationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => f.write_str("the key was only tapped, so nothing was transcribed"),
            Self::NothingHeard => f.write_str("no words were heard"),
            Self::Inserted { chars } => write!(f, "typed {chars} characters"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Listening,
    Finishing,
}

/// Push-to-talk dictation: record while the key is held, then type what was said into the
/// focused app.
///
/// [`Self::press`] starts recording. [`Self::release`] stops it at once and hands back the
/// [`Utterance`]; [`Self::finish`] transcribes and inserts it, which takes a network round trip,
/// so call it off the thread that delivers key events. Presses that arrive before the
/// utterance is finished or dropped are ignored. Safe to share between threads.
pub struct DictationService {
    input: Arc<dyn AudioInput>,
    speech: Arc<dyn SpeechToText>,
    inserter: Arc<dyn TextInserter>,
    phase: Arc<Mutex<Phase>>,
}

impl fmt::Debug for DictationService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DictationService")
            .field("phase", &*lock(&self.phase))
            .finish_non_exhaustive()
    }
}

impl DictationService {
    /// A service that records from `input`, transcribes with `speech`, and types through
    /// `inserter`.
    #[must_use]
    pub fn new(
        input: Arc<dyn AudioInput>,
        speech: Arc<dyn SpeechToText>,
        inserter: Arc<dyn TextInserter>,
    ) -> Self {
        Self {
            input,
            speech,
            inserter,
            phase: Arc::new(Mutex::new(Phase::Idle)),
        }
    }

    /// The dictation key went down: start recording unless already recording or finishing.
    ///
    /// # Errors
    ///
    /// [`DictationError::Capture`] if the microphone cannot be opened; the service stays
    /// idle, so the next press tries again.
    pub fn press(&self) -> Result<PressOutcome, DictationError> {
        let mut phase = lock(&self.phase);
        match *phase {
            Phase::Listening => Ok(PressOutcome::AlreadyListening),
            Phase::Finishing => Ok(PressOutcome::Busy),
            Phase::Idle => {
                self.input.start()?;
                *phase = Phase::Listening;
                Ok(PressOutcome::Listening)
            }
        }
    }

    /// The dictation key went up: stop recording. `None` when nothing was being recorded.
    #[must_use]
    pub fn release(&self) -> Option<Utterance> {
        let mut phase = lock(&self.phase);
        if *phase != Phase::Listening {
            return None;
        }
        *phase = Phase::Finishing;
        Some(Utterance {
            recorded: self.input.stop(),
            finishing: Finishing(Arc::clone(&self.phase)),
        })
    }

    /// Transcribes `utterance` and types the words, plus a space, into the focused app.
    ///
    /// # Errors
    ///
    /// Any [`DictationError`]. Either way the service is idle again afterwards.
    pub fn finish(&self, utterance: Utterance) -> Result<DictationOutcome, DictationError> {
        self.finish_then(utterance, |_| ())
    }

    /// [`DictationService::finish`], then `report` the result while the service still counts
    /// as finishing, so no new press starts recording until the report is done.
    ///
    /// # Errors
    ///
    /// Any [`DictationError`]. Either way the service is idle again afterwards.
    pub fn finish_then(
        &self,
        utterance: Utterance,
        report: impl FnOnce(&Result<DictationOutcome, DictationError>),
    ) -> Result<DictationOutcome, DictationError> {
        let Utterance {
            recorded,
            finishing,
        } = utterance;
        let result = self.type_out(recorded);
        report(&result);
        drop(finishing);
        result
    }

    fn type_out(
        &self,
        recorded: Result<AudioClip, CaptureError>,
    ) -> Result<DictationOutcome, DictationError> {
        let speech = recorded?;
        if speech.duration() < MIN_SPEECH {
            return Ok(DictationOutcome::TooShort);
        }
        let transcript = self.speech.transcribe(&speech)?;
        let text = transcript.trim();
        if text.is_empty() {
            return Ok(DictationOutcome::NothingHeard);
        }
        self.inserter.insert(&format!("{text} "))?;
        Ok(DictationOutcome::Inserted {
            chars: text.chars().count(),
        })
    }

    /// How lit the indicator should be, from 0 to 1, for what the microphone hears now.
    /// Zero when not recording.
    #[must_use]
    pub fn input_level(&self) -> f32 {
        if self.is_listening() {
            level_from_rms(self.input.level())
        } else {
            0.0
        }
    }

    /// Whether the microphone is recording.
    #[must_use]
    pub fn is_listening(&self) -> bool {
        *lock(&self.phase) == Phase::Listening
    }

    /// Whether a recording has stopped and is still being transcribed or typed.
    #[must_use]
    pub fn is_finishing(&self) -> bool {
        *lock(&self.phase) == Phase::Finishing
    }
}

/// A recording that has stopped and waits to be finished. Until it is finished or dropped,
/// new presses of the dictation key are ignored.
pub struct Utterance {
    recorded: Result<AudioClip, CaptureError>,
    finishing: Finishing,
}

impl fmt::Debug for Utterance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Utterance")
            .field("duration", &self.recorded.as_ref().map(AudioClip::duration))
            .finish_non_exhaustive()
    }
}

struct Finishing(Arc<Mutex<Phase>>);

impl Drop for Finishing {
    fn drop(&mut self) {
        *lock(&self.0) = Phase::Idle;
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Barrier, mpsc};
    use std::thread;

    use super::*;

    const RATE: u32 = 16_000;

    #[derive(Default)]
    struct Mic {
        starts: AtomicUsize,
        stops: AtomicUsize,
        refuse_start: bool,
        recorded: Option<Result<AudioClip, CaptureError>>,
        rms: f32,
    }

    impl AudioInput for Mic {
        fn start(&self) -> Result<(), CaptureError> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            if self.refuse_start {
                return Err(CaptureError("no microphone".into()));
            }
            Ok(())
        }

        fn stop(&self) -> Result<AudioClip, CaptureError> {
            self.stops.fetch_add(1, Ordering::SeqCst);
            self.recorded.clone().unwrap_or_else(|| Ok(speech(1_000)))
        }

        fn level(&self) -> f32 {
            self.rms
        }
    }

    struct Whisper {
        heard: Mutex<Vec<usize>>,
        answer: Result<String, TranscriptionError>,
    }

    impl Whisper {
        fn saying(text: &str) -> Self {
            Self {
                heard: Mutex::default(),
                answer: Ok(text.to_owned()),
            }
        }

        fn heard(&self) -> Vec<usize> {
            self.heard.lock().unwrap().clone()
        }
    }

    impl SpeechToText for Whisper {
        fn transcribe(&self, speech: &AudioClip) -> Result<String, TranscriptionError> {
            self.heard.lock().unwrap().push(speech.samples().len());
            self.answer.clone()
        }
    }

    #[derive(Default)]
    struct Typist {
        typed: Mutex<Vec<String>>,
        refuse: bool,
    }

    impl Typist {
        fn typed(&self) -> Vec<String> {
            self.typed.lock().unwrap().clone()
        }
    }

    impl TextInserter for Typist {
        fn insert(&self, text: &str) -> Result<(), InsertError> {
            if self.refuse {
                return Err(InsertError("input blocked".into()));
            }
            self.typed.lock().unwrap().push(text.to_owned());
            Ok(())
        }
    }

    fn speech(millis: u32) -> AudioClip {
        let samples = usize::try_from(RATE / 1_000 * millis).unwrap();
        AudioClip::from_samples(RATE, vec![0.1; samples]).unwrap()
    }

    struct Rig {
        mic: Arc<Mic>,
        whisper: Arc<Whisper>,
        typist: Arc<Typist>,
        service: DictationService,
    }

    fn rig(mic: Mic, whisper: Whisper, typist: Typist) -> Rig {
        let mic = Arc::new(mic);
        let whisper = Arc::new(whisper);
        let typist = Arc::new(typist);
        let service = DictationService::new(
            Arc::<Mic>::clone(&mic),
            Arc::<Whisper>::clone(&whisper),
            Arc::<Typist>::clone(&typist),
        );
        Rig {
            mic,
            whisper,
            typist,
            service,
        }
    }

    fn saying(text: &str) -> Rig {
        rig(Mic::default(), Whisper::saying(text), Typist::default())
    }

    fn recording(clip: Result<AudioClip, CaptureError>) -> Rig {
        let mic = Mic {
            recorded: Some(clip),
            ..Mic::default()
        };
        rig(mic, Whisper::saying("hello"), Typist::default())
    }

    fn dictate(rig: &Rig) -> Result<DictationOutcome, DictationError> {
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
        let utterance = rig.service.release().unwrap();
        rig.service.finish(utterance)
    }

    fn concurrent<T: Send>(action: impl Fn() -> T + Sync) -> Vec<T> {
        let start = Barrier::new(2);
        thread::scope(|scope| {
            let workers: Vec<_> = (0..2)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        action()
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect()
        })
    }

    struct Paused<T> {
        inner: T,
        entered: mpsc::Sender<()>,
        resume: Mutex<mpsc::Receiver<()>>,
    }

    struct Controller {
        entered: mpsc::Receiver<()>,
        resume: mpsc::Sender<()>,
    }

    impl<T> Paused<T> {
        fn new(inner: T) -> (Self, Controller) {
            let (entered, waiting) = mpsc::channel();
            let (resume, resumed) = mpsc::channel();
            (
                Self {
                    inner,
                    entered,
                    resume: Mutex::new(resumed),
                },
                Controller {
                    entered: waiting,
                    resume,
                },
            )
        }

        fn wait(&self) {
            self.entered.send(()).unwrap();
            self.resume
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
    }

    impl<T: SpeechToText> SpeechToText for Paused<T> {
        fn transcribe(&self, speech: &AudioClip) -> Result<String, TranscriptionError> {
            self.wait();
            self.inner.transcribe(speech)
        }
    }

    impl<T: TextInserter> TextInserter for Paused<T> {
        fn insert(&self, text: &str) -> Result<(), InsertError> {
            self.wait();
            self.inner.insert(text)
        }
    }

    impl Controller {
        fn assert_busy_and_resume(&self, service: &DictationService, mic: &Mic) {
            self.entered.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(service.press(), Ok(PressOutcome::Busy));
            assert!(service.release().is_none());
            assert!(!service.is_listening());
            assert_eq!(mic.starts.load(Ordering::SeqCst), 1);
            assert_eq!(mic.stops.load(Ordering::SeqCst), 1);
            self.resume.send(()).unwrap();
        }
    }

    #[test]
    fn the_input_level_follows_the_microphone_only_while_listening() {
        let mic = Mic {
            rms: 0.031_623,
            ..Mic::default()
        };
        let rig = rig(mic, Whisper::saying("hi"), Typist::default());
        assert!(rig.service.input_level().abs() < f32::EPSILON, "idle");
        rig.service.press().unwrap();
        assert!(
            (rig.service.input_level() - 0.5).abs() < 1e-3,
            "-30 dB is half lit"
        );
        drop(rig.service.release());
        assert!(rig.service.input_level().abs() < f32::EPSILON, "released");
    }

    #[test]
    fn simultaneous_presses_open_the_microphone_only_once() {
        let rig = saying("hi");
        let outcomes = concurrent(|| rig.service.press().unwrap());
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == PressOutcome::Listening)
                .count(),
            1
        );
        assert!(outcomes.contains(&PressOutcome::AlreadyListening));
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 1);
        drop(rig.service.release());
    }

    #[test]
    fn simultaneous_releases_hand_out_only_one_utterance() {
        let rig = saying("hi");
        rig.service.press().unwrap();
        let released = concurrent(|| rig.service.release());
        assert_eq!(released.iter().filter(|item| item.is_some()).count(), 1);
        assert_eq!(rig.mic.stops.load(Ordering::SeqCst), 1);
        assert_eq!(rig.service.press(), Ok(PressOutcome::Busy));
        drop(released);
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
        drop(rig.service.release());
    }

    #[test]
    fn key_events_stay_busy_during_transcription_and_insertion() {
        let mic = Arc::new(Mic::default());
        let (whisper, transcribing) = Paused::new(Whisper::saying("hi"));
        let (typist, inserting) = Paused::new(Typist::default());
        let typist = Arc::new(typist);
        let service = DictationService::new(
            Arc::<Mic>::clone(&mic),
            Arc::new(whisper),
            Arc::<Paused<Typist>>::clone(&typist),
        );
        service.press().unwrap();
        let utterance = service.release().unwrap();
        thread::scope(|scope| {
            let worker = scope.spawn(|| service.finish(utterance));
            transcribing.assert_busy_and_resume(&service, &mic);
            inserting.assert_busy_and_resume(&service, &mic);
            assert_eq!(
                worker.join().unwrap(),
                Ok(DictationOutcome::Inserted { chars: 2 })
            );
        });
        assert_eq!(typist.inner.typed(), ["hi "]);
        assert_eq!(service.press(), Ok(PressOutcome::Listening));
        drop(service.release());
    }

    #[test]
    fn hold_speak_release_types_the_words_and_a_space() {
        let rig = saying("Hello, world.");
        assert_eq!(dictate(&rig), Ok(DictationOutcome::Inserted { chars: 13 }));
        assert_eq!(rig.typist.typed(), ["Hello, world. "]);
        assert_eq!(rig.whisper.heard(), [16_000]);
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 1);
        assert_eq!(rig.mic.stops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn the_transcript_is_trimmed_and_counted_in_characters() {
        let rig = saying("  café crème \n");
        assert_eq!(dictate(&rig), Ok(DictationOutcome::Inserted { chars: 10 }));
        assert_eq!(rig.typist.typed(), ["café crème "]);
    }

    #[test]
    fn the_service_listens_only_between_press_and_release() {
        let rig = saying("hi");
        let phase = |service: &DictationService| (service.is_listening(), service.is_finishing());
        let mut phases = vec![phase(&rig.service)];
        rig.service.press().unwrap();
        phases.push(phase(&rig.service));
        let utterance = rig.service.release().unwrap();
        phases.push(phase(&rig.service));
        rig.service.finish(utterance).unwrap();
        phases.push(phase(&rig.service));
        assert_eq!(
            phases,
            [(false, false), (true, false), (false, true), (false, false)]
        );
    }

    #[test]
    fn the_microphone_stops_on_release() {
        let rig = saying("hi");
        rig.service.press().unwrap();
        assert_eq!(rig.mic.stops.load(Ordering::SeqCst), 0);
        drop(rig.service.release());
        assert_eq!(rig.mic.stops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_held_key_repeating_does_not_restart_the_recording() {
        let rig = saying("hi");
        rig.service.press().unwrap();
        assert_eq!(rig.service.press(), Ok(PressOutcome::AlreadyListening));
        assert_eq!(rig.service.press(), Ok(PressOutcome::AlreadyListening));
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_press_while_the_last_dictation_finishes_is_ignored() {
        let rig = saying("hi");
        rig.service.press().unwrap();
        let utterance = rig.service.release().unwrap();
        assert_eq!(rig.service.press(), Ok(PressOutcome::Busy));
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 1);
        assert!(
            rig.service.release().is_none(),
            "the ignored press recorded nothing"
        );
        rig.service.finish(utterance).unwrap();
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn dropping_an_unfinished_utterance_makes_the_service_idle() {
        let rig = saying("hi");
        rig.service.press().unwrap();
        drop(rig.service.release());
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
        assert!(rig.typist.typed().is_empty(), "nothing was typed");
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let rig = saying("hi");
        assert!(rig.service.release().is_none());
        assert_eq!(rig.mic.stops.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_tap_shorter_than_a_quarter_second_is_not_transcribed() {
        let rig = recording(Ok(speech(249)));
        assert_eq!(dictate(&rig), Ok(DictationOutcome::TooShort));
        assert!(rig.whisper.heard().is_empty(), "Whisper was not asked");
        assert!(rig.typist.typed().is_empty(), "nothing was typed");
        assert_eq!(MIN_SPEECH, Duration::from_millis(250));
    }

    #[test]
    fn a_quarter_second_is_long_enough() {
        let rig = recording(Ok(speech(250)));
        assert_eq!(dictate(&rig), Ok(DictationOutcome::Inserted { chars: 5 }));
        assert_eq!(rig.whisper.heard(), [4_000]);
    }

    #[test]
    fn silence_types_nothing() {
        for transcript in ["", "  \n "] {
            let rig = saying(transcript);
            assert_eq!(dictate(&rig), Ok(DictationOutcome::NothingHeard));
            assert!(rig.typist.typed().is_empty(), "nothing was typed");
        }
    }

    #[test]
    fn a_missing_microphone_fails_the_press_and_the_next_press_tries_again() {
        let mic = Mic {
            refuse_start: true,
            ..Mic::default()
        };
        let rig = rig(mic, Whisper::saying("hi"), Typist::default());
        let error = rig.service.press().unwrap_err();
        assert_eq!(
            error,
            DictationError::Capture(CaptureError("no microphone".into()))
        );
        assert_eq!(
            error.to_string(),
            "can't record from the microphone: no microphone"
        );
        assert!(!rig.service.is_listening(), "still idle");
        assert!(rig.service.release().is_none(), "nothing to release");
        rig.service.press().unwrap_err();
        assert_eq!(rig.mic.starts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_unusable_recording_is_reported_and_the_service_is_idle_again() {
        let rig = recording(Err(CaptureError("device unplugged".into())));
        assert_eq!(
            dictate(&rig),
            Err(DictationError::Capture(CaptureError(
                "device unplugged".into()
            )))
        );
        assert!(rig.whisper.heard().is_empty(), "Whisper was not asked");
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
    }

    #[test]
    fn a_transcription_failure_is_reported_and_the_service_is_idle_again() {
        let whisper = Whisper {
            heard: Mutex::default(),
            answer: Err(TranscriptionError::Server("no model".into())),
        };
        let rig = rig(Mic::default(), whisper, Typist::default());
        let error = dictate(&rig).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the speech recognition server reported an error: no model"
        );
        assert!(
            matches!(error, DictationError::Transcription(_)),
            "{error:?}"
        );
        assert!(rig.typist.typed().is_empty(), "nothing was typed");
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
    }

    #[test]
    fn an_insertion_failure_is_reported_and_the_service_is_idle_again() {
        let typist = Typist {
            refuse: true,
            ..Typist::default()
        };
        let rig = rig(Mic::default(), Whisper::saying("hi"), typist);
        let error = dictate(&rig).unwrap_err();
        assert_eq!(
            error,
            DictationError::Insertion(InsertError("input blocked".into()))
        );
        assert_eq!(
            error.to_string(),
            "could not type into the focused app: input blocked"
        );
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
    }

    #[test]
    fn outcomes_describe_themselves_without_the_words() {
        let presses = [
            PressOutcome::Listening,
            PressOutcome::AlreadyListening,
            PressOutcome::Busy,
        ]
        .map(|outcome| outcome.to_string());
        assert_eq!(
            presses,
            [
                "listening",
                "already listening",
                "still typing the last dictation, so this press is ignored"
            ]
        );
        let endings = [
            DictationOutcome::TooShort,
            DictationOutcome::NothingHeard,
            DictationOutcome::Inserted { chars: 12 },
        ]
        .map(|outcome| outcome.to_string());
        assert_eq!(
            endings,
            [
                "the key was only tapped, so nothing was transcribed",
                "no words were heard",
                "typed 12 characters"
            ]
        );
    }

    #[test]
    fn debug_output_shows_the_phase_and_the_recording_length() {
        let rig = saying("hi");
        assert_eq!(
            format!("{:?}", rig.service),
            "DictationService { phase: Idle, .. }"
        );
        rig.service.press().unwrap();
        assert_eq!(
            format!("{:?}", rig.service),
            "DictationService { phase: Listening, .. }"
        );
        let utterance = rig.service.release().unwrap();
        assert_eq!(
            format!("{utterance:?}"),
            "Utterance { duration: Ok(1s), .. }"
        );
        assert_eq!(
            format!("{:?}", rig.service),
            "DictationService { phase: Finishing, .. }"
        );
    }

    #[test]
    fn the_report_runs_while_a_new_press_is_still_ignored() {
        let rig = saying("hello");
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
        let utterance = rig.service.release().unwrap();
        let mut seen = None;
        let result = rig.service.finish_then(utterance, |result| {
            seen = Some((result.clone(), rig.service.press()));
        });
        assert_eq!(result, Ok(DictationOutcome::Inserted { chars: 5 }));
        assert_eq!(seen, Some((result, Ok(PressOutcome::Busy))));
        assert_eq!(rig.service.press(), Ok(PressOutcome::Listening));
    }
}
