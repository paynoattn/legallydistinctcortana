use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use speech_domain::OverlayEvent;

use crate::overlay::{OverlayController, finish_event, press_event};
use crate::{DictationError, DictationOutcome, DictationService, PressOutcome};

/// What push-to-talk needs from the app around it.
pub trait DictationHost: Send + Sync {
    /// The dictation service for the current settings.
    fn dictation(&self) -> Arc<DictationService>;
    /// Shows `level` on the indicator about 30 times a second, until it returns `None`.
    fn follow_level(&self, level: Box<dyn Fn() -> Option<f32> + Send>);
    /// Runs `task` on another thread.
    fn spawn(&self, task: Box<dyn FnOnce() + Send>);
    /// Records how a press went.
    fn pressed(&self, result: &Result<PressOutcome, DictationError>);
    /// Records how a dictation ended.
    fn finished(&self, result: &Result<DictationOutcome, DictationError>);
}

/// Hold the dictation key to record, release it to type what was said, with the indicator
/// following along. A press while the last dictation is still being typed goes to the same
/// service, which ignores it, even if the settings have changed since. Send [`PushToTalk::key`] calls in order from one thread, since a press
/// waits for the microphone to open; [`PushToTalk::announce`] never waits.
pub struct PushToTalk {
    host: Arc<dyn DictationHost>,
    overlay: Arc<OverlayController>,
    active: Mutex<Option<Arc<DictationService>>>,
}

impl fmt::Debug for PushToTalk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PushToTalk")
            .field("overlay", &self.overlay)
            .finish_non_exhaustive()
    }
}

impl PushToTalk {
    /// Push-to-talk that takes its service from `host` and shows progress on `overlay`.
    #[must_use]
    pub fn new(host: Arc<dyn DictationHost>, overlay: Arc<OverlayController>) -> Self {
        Self {
            host,
            overlay,
            active: Mutex::default(),
        }
    }

    /// Shows a press on the indicator straight away, before the microphone opens.
    pub fn announce(&self, pressed: bool) {
        if pressed {
            self.overlay.handle(OverlayEvent::KeyDown);
        }
    }

    /// The key went down (`pressed`) or up. A press can block while the microphone opens;
    /// a release hands the transcription to another thread.
    pub fn key(&self, pressed: bool) {
        if pressed {
            self.press();
        } else {
            self.release();
        }
    }

    fn press(&self) {
        let service = self.service_for_press();
        let result = service.press();
        self.host.pressed(&result);
        if let Some(event) = press_event(&result) {
            self.overlay.handle(event);
        }
        if result == Ok(PressOutcome::Listening) {
            self.host.follow_level(Box::new(move || {
                service.is_listening().then(|| service.input_level())
            }));
        }
    }

    /// The service still recording or typing, so the press goes where the release will;
    /// otherwise the host's current one, which follows the latest settings.
    fn service_for_press(&self) -> Arc<DictationService> {
        let mut active = lock(&self.active);
        let service = match active.take() {
            Some(busy) if busy.is_listening() || busy.is_finishing() => busy,
            _ => self.host.dictation(),
        };
        *active = Some(Arc::clone(&service));
        service
    }

    fn release(&self) {
        let Some(service) = lock(&self.active).clone() else {
            return;
        };
        let Some(utterance) = service.release() else {
            return;
        };
        self.overlay.handle(OverlayEvent::KeyUp);
        let host = Arc::clone(&self.host);
        let overlay = Arc::clone(&self.overlay);
        self.host.spawn(Box::new(move || {
            drop(service.finish_then(utterance, |result| {
                overlay.handle(finish_event(result));
                host.finished(result);
            }));
        }));
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use platform_ports::{InsertError, TextInserter};
    use speech_domain::{AudioClip, OverlayState, Stage};

    use super::*;
    use crate::ports::{AudioInput, CaptureError, SpeechToText, TranscriptionError};
    use crate::{Delay, OverlaySurface};

    type Task = Box<dyn FnOnce() + Send>;
    type Level = Box<dyn Fn() -> Option<f32> + Send>;

    struct Mic {
        refuse: bool,
    }

    impl AudioInput for Mic {
        fn start(&self) -> Result<(), CaptureError> {
            if self.refuse {
                return Err(CaptureError("no microphone".into()));
            }
            Ok(())
        }

        fn stop(&self) -> Result<AudioClip, CaptureError> {
            Ok(AudioClip::from_samples(16_000, vec![0.1; 16_000]).unwrap())
        }

        fn level(&self) -> f32 {
            0.01
        }
    }

    struct Whisper;

    impl SpeechToText for Whisper {
        fn transcribe(&self, _speech: &AudioClip) -> Result<String, TranscriptionError> {
            Ok("hello".into())
        }
    }

    struct Typist;

    impl TextInserter for Typist {
        fn insert(&self, _text: &str) -> Result<(), InsertError> {
            Ok(())
        }
    }

    fn service(refuse: bool) -> Arc<DictationService> {
        Arc::new(DictationService::new(
            Arc::new(Mic { refuse }),
            Arc::new(Whisper),
            Arc::new(Typist),
        ))
    }

    #[derive(Default)]
    struct Host {
        services: Mutex<Vec<Arc<DictationService>>>,
        built: Mutex<usize>,
        levels: Mutex<Vec<Level>>,
        tasks: Mutex<Vec<Task>>,
        presses: Mutex<Vec<Result<PressOutcome, DictationError>>>,
        finishes: Mutex<Vec<Result<DictationOutcome, DictationError>>>,
    }

    impl Host {
        fn with(services: Vec<Arc<DictationService>>) -> Self {
            Self {
                services: Mutex::new(services),
                ..Self::default()
            }
        }

        fn run_tasks(&self) {
            let tasks: Vec<_> = self.tasks.lock().unwrap().drain(..).collect();
            for task in tasks {
                task();
            }
        }

        fn sample_levels(&self) -> Vec<Option<f32>> {
            self.levels
                .lock()
                .unwrap()
                .iter()
                .map(|level| level())
                .collect()
        }
    }

    impl DictationHost for Host {
        fn dictation(&self) -> Arc<DictationService> {
            *self.built.lock().unwrap() += 1;
            self.services.lock().unwrap().remove(0)
        }

        fn follow_level(&self, level: Level) {
            self.levels.lock().unwrap().push(level);
        }

        fn spawn(&self, task: Task) {
            self.tasks.lock().unwrap().push(task);
        }

        fn pressed(&self, result: &Result<PressOutcome, DictationError>) {
            self.presses.lock().unwrap().push(result.clone());
        }

        fn finished(&self, result: &Result<DictationOutcome, DictationError>) {
            self.finishes.lock().unwrap().push(result.clone());
        }
    }

    #[derive(Default)]
    struct Screen(Mutex<Vec<OverlayState>>);

    impl OverlaySurface for Screen {
        fn show(&self, state: &OverlayState) {
            self.0.lock().unwrap().push(state.clone());
        }
    }

    struct Never;

    impl Delay for Never {
        fn after(&self, _wait: Duration, _task: Task) {}
    }

    struct Rig {
        host: Arc<Host>,
        screen: Arc<Screen>,
        talk: PushToTalk,
    }

    impl Rig {
        fn shown(&self) -> Vec<OverlayState> {
            self.screen.0.lock().unwrap().clone()
        }
    }

    fn rig(services: Vec<Arc<DictationService>>) -> Rig {
        let host = Arc::new(Host::with(services));
        let screen = Arc::new(Screen::default());
        let overlay = OverlayController::new(Arc::<Screen>::clone(&screen), Arc::new(Never));
        let talk = PushToTalk::new(Arc::<Host>::clone(&host), overlay);
        Rig { host, screen, talk }
    }

    fn hold_and_release(rig: &Rig) {
        rig.talk.announce(true);
        rig.talk.key(true);
        rig.talk.announce(false);
        rig.talk.key(false);
    }

    #[test]
    fn a_dictation_shows_every_stage_and_types_in_the_background() {
        let rig = rig(vec![service(false)]);
        hold_and_release(&rig);
        assert_eq!(*rig.host.finishes.lock().unwrap(), []);
        rig.host.run_tasks();
        assert_eq!(
            rig.shown(),
            [
                OverlayState::Dictating(Stage::Armed),
                OverlayState::Dictating(Stage::Listening),
                OverlayState::Dictating(Stage::Thinking),
                OverlayState::Inserted,
            ]
        );
        assert_eq!(
            *rig.host.presses.lock().unwrap(),
            [Ok(PressOutcome::Listening)]
        );
        assert_eq!(
            *rig.host.finishes.lock().unwrap(),
            [Ok(DictationOutcome::Inserted { chars: 5 })]
        );
    }

    #[test]
    fn a_press_before_the_last_dictation_is_typed_goes_to_the_same_service() {
        let rig = rig(vec![service(false), service(false)]);
        rig.talk.key(true);
        rig.talk.key(true);
        rig.talk.key(false);
        rig.talk.key(true);
        rig.talk.key(false);
        assert_eq!(*rig.host.built.lock().unwrap(), 1);
        rig.host.run_tasks();
        rig.talk.key(true);
        assert_eq!(*rig.host.built.lock().unwrap(), 2);
        assert_eq!(
            *rig.host.presses.lock().unwrap(),
            [
                Ok(PressOutcome::Listening),
                Ok(PressOutcome::AlreadyListening),
                Ok(PressOutcome::Busy),
                Ok(PressOutcome::Listening),
            ]
        );
    }

    #[test]
    fn a_dictation_after_the_last_one_was_typed_takes_the_current_service_and_finishes() {
        let first = service(false);
        let second = service(false);
        let rig = rig(vec![Arc::clone(&first), Arc::clone(&second)]);
        hold_and_release(&rig);
        rig.host.run_tasks();
        hold_and_release(&rig);
        assert!(
            !second.is_listening(),
            "the second release stopped the microphone"
        );
        rig.host.run_tasks();
        assert_eq!(*rig.host.built.lock().unwrap(), 2);
        assert_eq!(
            *rig.host.finishes.lock().unwrap(),
            [
                Ok(DictationOutcome::Inserted { chars: 5 }),
                Ok(DictationOutcome::Inserted { chars: 5 }),
            ]
        );
        assert!(!first.is_finishing() && !second.is_finishing());
    }

    #[test]
    fn a_failed_press_lets_the_next_press_take_the_current_service() {
        let rig = rig(vec![service(true), service(false)]);
        hold_and_release(&rig);
        rig.talk.key(true);
        assert_eq!(*rig.host.built.lock().unwrap(), 2);
        assert_eq!(
            rig.host.presses.lock().unwrap().last(),
            Some(&Ok(PressOutcome::Listening))
        );
    }

    #[test]
    fn the_level_is_followed_only_while_recording() {
        let rig = rig(vec![service(false)]);
        rig.talk.key(true);
        assert_eq!(rig.host.sample_levels(), [Some(0.25)]);
        rig.talk.key(false);
        assert_eq!(rig.host.sample_levels(), [None]);
    }

    #[test]
    fn a_microphone_that_will_not_open_shows_an_error_and_follows_nothing() {
        let rig = rig(vec![service(true)]);
        hold_and_release(&rig);
        assert_eq!(rig.shown().len(), 2);
        assert_eq!(rig.shown()[1].id(), "error");
        assert_eq!(rig.host.levels.lock().unwrap().len(), 0);
        assert_eq!(rig.host.tasks.lock().unwrap().len(), 0);
        assert!(
            format!("{:?}", rig.talk).contains("Error"),
            "debug names the state"
        );
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let rig = rig(vec![]);
        rig.talk.announce(false);
        rig.talk.key(false);
        assert_eq!(rig.shown(), []);
        assert_eq!(*rig.host.built.lock().unwrap(), 0);
    }
}
