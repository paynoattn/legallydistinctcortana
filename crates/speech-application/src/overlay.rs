use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use speech_domain::{Fix, OverlayError, OverlayEvent, OverlayState};

use crate::ports::TranscriptionError;
use crate::{DictationError, DictationOutcome, PressOutcome};

/// Where the indicator is drawn: a window in the corner of the screen.
pub trait OverlaySurface: Send + Sync {
    /// Shows `state`. Called in the order the states happen, so it must not block on the
    /// thread that delivers key events; queue the work instead.
    fn show(&self, state: &OverlayState);
}

/// Runs a task later, off the caller's thread.
pub trait Delay: Send + Sync {
    /// Runs `task` once, after `wait`, on another thread (never before returning).
    fn after(&self, wait: Duration, task: Box<dyn FnOnce() + Send>);
}

struct Current {
    state: OverlayState,
    generation: u64,
    reading: bool,
}

/// Keeps the corner indicator in step with dictation and reading: applies each
/// [`OverlayEvent`] to the current [`OverlayState`], shows every change, and ends the
/// states that linger (the check after inserting, an error) when their time runs out,
/// unless something else happened first. Safe to share between threads.
pub struct OverlayController {
    surface: Arc<dyn OverlaySurface>,
    delay: Arc<dyn Delay>,
    current: Mutex<Current>,
    me: Weak<Self>,
}

impl fmt::Debug for OverlayController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverlayController")
            .field("state", &lock(&self.current).state)
            .finish_non_exhaustive()
    }
}

impl OverlayController {
    /// A controller that starts hidden and draws on `surface`, timing lingering states
    /// with `delay`.
    #[must_use]
    pub fn new(surface: Arc<dyn OverlaySurface>, delay: Arc<dyn Delay>) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            surface,
            delay,
            current: Mutex::new(Current {
                state: OverlayState::Hidden,
                generation: 0,
                reading: false,
            }),
            me: Weak::clone(me),
        })
    }

    /// Applies `event`, showing the new state if it changed.
    pub fn handle(&self, event: OverlayEvent) {
        self.apply(&mut lock(&self.current), event);
    }

    /// The person clicked the error away. Returns the fix it offered, so the caller can
    /// carry it out.
    pub fn dismiss(&self) -> Option<Fix> {
        let mut current = lock(&self.current);
        let fix = current.state.error().and_then(|error| error.fix);
        self.apply(&mut current, OverlayEvent::Dismissed);
        fix
    }

    /// Shows the current state again, for example after the indicator's preferences
    /// changed.
    pub fn redraw(&self) {
        let current = lock(&self.current);
        self.surface.show(&current.state);
    }

    /// What the indicator shows now.
    #[must_use]
    pub fn state(&self) -> OverlayState {
        lock(&self.current).state.clone()
    }

    fn expire(&self, generation: u64) {
        let mut current = lock(&self.current);
        if current.generation == generation {
            self.apply(&mut current, OverlayEvent::Elapsed);
        }
    }

    fn apply(&self, current: &mut Current, event: OverlayEvent) {
        current.reading = event.reading(current.reading);
        let next = current.state.after(event).or_speaking(current.reading);
        if next == current.state {
            return;
        }
        current.generation += 1;
        self.surface.show(&next);
        if let Some(wait) = next.linger() {
            self.expire_later(wait, current.generation);
        }
        current.state = next;
    }

    fn expire_later(&self, wait: Duration, generation: u64) {
        let me = Weak::clone(&self.me);
        self.delay.after(
            wait,
            Box::new(move || {
                if let Some(controller) = me.upgrade() {
                    controller.expire(generation);
                }
            }),
        );
    }
}

/// The indicator event for pressing the dictation key, if it changes anything: the
/// microphone opened, or it failed.
#[must_use]
pub fn press_event(result: &Result<PressOutcome, DictationError>) -> Option<OverlayEvent> {
    match result {
        Ok(PressOutcome::Listening) => Some(OverlayEvent::Recording),
        Ok(PressOutcome::AlreadyListening | PressOutcome::Busy) => None,
        Err(error) => Some(OverlayEvent::Failed(overlay_error(error))),
    }
}

/// The indicator event for a finished dictation.
#[must_use]
pub fn finish_event(result: &Result<DictationOutcome, DictationError>) -> OverlayEvent {
    match result {
        Ok(DictationOutcome::Inserted { .. }) => OverlayEvent::Inserted,
        Ok(DictationOutcome::TooShort | DictationOutcome::NothingHeard) => OverlayEvent::Discarded,
        Err(error) => OverlayEvent::Failed(overlay_error(error)),
    }
}

/// The one line the indicator shows for `error`, and the fix it offers.
#[must_use]
pub fn overlay_error(error: &DictationError) -> OverlayError {
    match error {
        DictationError::Capture(_) => OverlayError {
            reason: "Can't use the microphone.".into(),
            fix: Some(Fix::SoundSettings),
        },
        DictationError::Transcription(TranscriptionError::Unreachable { address, .. }) => {
            OverlayError {
                reason: format!("Can't reach speech recognition at {address}."),
                fix: Some(Fix::Settings),
            }
        }
        DictationError::Transcription(_) => OverlayError {
            reason: "Speech recognition failed.".into(),
            fix: None,
        },
        DictationError::Insertion(_) => OverlayError {
            reason: "Couldn't type into this app.".into(),
            fix: None,
        },
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use platform_ports::InsertError;

    use super::*;
    use crate::ports::CaptureError;
    use speech_domain::Stage;

    #[derive(Default)]
    struct Screen(Mutex<Vec<OverlayState>>);

    impl Screen {
        fn shown(&self) -> Vec<OverlayState> {
            self.0.lock().unwrap().clone()
        }
    }

    impl OverlaySurface for Screen {
        fn show(&self, state: &OverlayState) {
            self.0.lock().unwrap().push(state.clone());
        }
    }

    type Task = Box<dyn FnOnce() + Send>;

    #[derive(Default)]
    struct ManualDelay(Mutex<Vec<(Duration, Task)>>);

    impl ManualDelay {
        fn waits(&self) -> Vec<Duration> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|(wait, _)| *wait)
                .collect()
        }

        fn run_all(&self) {
            let tasks: Vec<_> = self.0.lock().unwrap().drain(..).collect();
            for (_, task) in tasks {
                task();
            }
        }
    }

    impl Delay for ManualDelay {
        fn after(&self, wait: Duration, task: Task) {
            self.0.lock().unwrap().push((wait, task));
        }
    }

    fn controller() -> (Arc<OverlayController>, Arc<Screen>, Arc<ManualDelay>) {
        let screen = Arc::new(Screen::default());
        let delay = Arc::new(ManualDelay::default());
        let surface: Arc<dyn OverlaySurface> = Arc::<Screen>::clone(&screen);
        let timer: Arc<dyn Delay> = Arc::<ManualDelay>::clone(&delay);
        let controller = OverlayController::new(surface, timer);
        (controller, screen, delay)
    }

    fn unreachable() -> DictationError {
        DictationError::Transcription(TranscriptionError::Unreachable {
            address: "host:10300".into(),
            reason: "refused".into(),
        })
    }

    #[test]
    fn every_change_is_shown_in_order() {
        let (overlay, screen, _) = controller();
        for event in [
            OverlayEvent::KeyDown,
            OverlayEvent::Recording,
            OverlayEvent::KeyUp,
        ] {
            overlay.handle(event);
        }
        assert_eq!(
            screen.shown(),
            [
                OverlayState::Dictating(Stage::Armed),
                OverlayState::Dictating(Stage::Listening),
                OverlayState::Dictating(Stage::Thinking)
            ]
        );
        assert_eq!(overlay.state(), OverlayState::Dictating(Stage::Thinking));
    }

    #[test]
    fn repeats_that_change_nothing_are_not_shown_again() {
        let (overlay, screen, _) = controller();
        overlay.handle(OverlayEvent::KeyDown);
        overlay.handle(OverlayEvent::Recording);
        overlay.handle(OverlayEvent::KeyDown);
        assert_eq!(
            screen.shown(),
            [
                OverlayState::Dictating(Stage::Armed),
                OverlayState::Dictating(Stage::Listening)
            ]
        );
    }

    #[test]
    fn inserted_hides_itself_after_its_linger_time() {
        let (overlay, screen, delay) = controller();
        overlay.handle(OverlayEvent::KeyDown);
        overlay.handle(OverlayEvent::KeyUp);
        overlay.handle(OverlayEvent::Inserted);
        assert_eq!(delay.waits(), [Duration::from_millis(700)]);
        delay.run_all();
        assert_eq!(overlay.state(), OverlayState::Hidden);
        assert_eq!(screen.shown().last(), Some(&OverlayState::Hidden));
    }

    #[test]
    fn a_new_press_cancels_the_pending_hide() {
        let (overlay, _, delay) = controller();
        overlay.handle(OverlayEvent::KeyDown);
        overlay.handle(OverlayEvent::KeyUp);
        overlay.handle(OverlayEvent::Inserted);
        overlay.handle(OverlayEvent::KeyDown);
        delay.run_all();
        assert_eq!(overlay.state(), OverlayState::Dictating(Stage::Armed));
    }

    #[test]
    fn a_second_error_restarts_the_error_time() {
        let (overlay, _, delay) = controller();
        let first = overlay_error(&unreachable());
        let second = overlay_error(&DictationError::Capture(CaptureError("gone".into())));
        overlay.handle(OverlayEvent::Failed(first));
        overlay.handle(OverlayEvent::Failed(second.clone()));
        assert_eq!(
            delay.waits(),
            [Duration::from_secs(6), Duration::from_secs(6)]
        );
        let tasks: Vec<_> = delay.0.lock().unwrap().drain(..).collect();
        let mut tasks = tasks.into_iter().map(|(_, task)| task);
        tasks.next().unwrap()();
        assert_eq!(overlay.state(), OverlayState::Error(second));
        tasks.next().unwrap()();
        assert_eq!(overlay.state(), OverlayState::Hidden);
    }

    #[test]
    fn dismissing_an_error_hides_it_and_hands_back_its_fix() {
        let (overlay, _, _) = controller();
        overlay.handle(OverlayEvent::Failed(overlay_error(&unreachable())));
        assert_eq!(overlay.dismiss(), Some(Fix::Settings));
        assert_eq!(overlay.state(), OverlayState::Hidden);
        assert_eq!(overlay.dismiss(), None);
    }

    #[test]
    fn a_reading_shows_again_after_a_dictation_that_interrupted_it() {
        let (overlay, screen, delay) = controller();
        for event in [
            OverlayEvent::SpeakingStarted,
            OverlayEvent::KeyDown,
            OverlayEvent::KeyUp,
            OverlayEvent::Inserted,
        ] {
            overlay.handle(event);
        }
        delay.run_all();
        assert_eq!(overlay.state(), OverlayState::Speaking);
        overlay.handle(OverlayEvent::SpeakingEnded);
        assert_eq!(overlay.state(), OverlayState::Hidden);
        assert_eq!(screen.shown().len(), 6);
    }

    #[test]
    fn a_reading_that_ends_during_a_dictation_is_not_shown_again() {
        let (overlay, _, _) = controller();
        for event in [
            OverlayEvent::SpeakingStarted,
            OverlayEvent::KeyDown,
            OverlayEvent::SpeakingEnded,
            OverlayEvent::Discarded,
        ] {
            overlay.handle(event);
        }
        assert_eq!(overlay.state(), OverlayState::Hidden);
    }

    #[test]
    fn redraw_shows_the_current_state_again() {
        let (overlay, screen, _) = controller();
        overlay.handle(OverlayEvent::SpeakingStarted);
        overlay.redraw();
        assert_eq!(
            screen.shown(),
            [OverlayState::Speaking, OverlayState::Speaking]
        );
    }

    #[test]
    fn states_that_wait_for_something_schedule_nothing() {
        let (overlay, _, delay) = controller();
        overlay.handle(OverlayEvent::KeyDown);
        overlay.handle(OverlayEvent::SpeakingStarted);
        assert!(delay.waits().is_empty(), "nothing lingers");
    }

    #[test]
    fn a_dropped_controller_ignores_its_timers() {
        let (overlay, screen, delay) = controller();
        overlay.handle(OverlayEvent::Failed(overlay_error(&unreachable())));
        drop(overlay);
        delay.run_all();
        assert_eq!(screen.shown().len(), 1);
    }

    #[test]
    fn debug_shows_the_state() {
        let (overlay, _, _) = controller();
        assert!(
            format!("{overlay:?}").contains("Hidden"),
            "debug output names the state"
        );
    }

    #[test]
    fn a_press_that_opens_the_microphone_starts_listening() {
        assert_eq!(
            press_event(&Ok(PressOutcome::Listening)),
            Some(OverlayEvent::Recording)
        );
        assert_eq!(press_event(&Ok(PressOutcome::AlreadyListening)), None);
        assert_eq!(press_event(&Ok(PressOutcome::Busy)), None);
    }

    #[test]
    fn a_press_without_a_microphone_offers_sound_settings() {
        let event = press_event(&Err(DictationError::Capture(CaptureError(
            "no device".into(),
        ))));
        assert_eq!(
            event,
            Some(OverlayEvent::Failed(OverlayError {
                reason: "Can't use the microphone.".into(),
                fix: Some(Fix::SoundSettings),
            }))
        );
    }

    #[test]
    fn a_finished_dictation_is_inserted_or_discarded() {
        assert_eq!(
            finish_event(&Ok(DictationOutcome::Inserted { chars: 3 })),
            OverlayEvent::Inserted
        );
        assert_eq!(
            finish_event(&Ok(DictationOutcome::TooShort)),
            OverlayEvent::Discarded
        );
        assert_eq!(
            finish_event(&Ok(DictationOutcome::NothingHeard)),
            OverlayEvent::Discarded
        );
    }

    #[test]
    fn an_unreachable_server_names_its_address_and_offers_settings() {
        assert_eq!(
            finish_event(&Err(unreachable())),
            OverlayEvent::Failed(OverlayError {
                reason: "Can't reach speech recognition at host:10300.".into(),
                fix: Some(Fix::Settings),
            })
        );
    }

    #[test]
    fn other_failures_explain_without_a_fix() {
        assert_eq!(
            overlay_error(&DictationError::Transcription(TranscriptionError::Server(
                "busy".into()
            ))),
            OverlayError {
                reason: "Speech recognition failed.".into(),
                fix: None,
            }
        );
        assert_eq!(
            overlay_error(&DictationError::Insertion(InsertError("blocked".into()))),
            OverlayError {
                reason: "Couldn't type into this app.".into(),
                fix: None,
            }
        );
    }
}
