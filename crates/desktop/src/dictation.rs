//! Wiring for push-to-talk dictation: hold the shortcut to record, release it to type what
//! was said into the focused app. Key events are handled in order on one worker thread, so
//! opening the microphone never blocks the thread that draws the indicator. The behavior
//! lives in `speech_application::PushToTalk`.

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;

use speech_application::{
    DictationError, DictationHost, DictationOutcome, DictationService, OverlayController,
    PressOutcome, PushToTalk,
};
use tauri::{AppHandle, Manager, Runtime};

use crate::overlay;
use crate::support::{self, log_failure};
use crate::voice::Voice;

pub(crate) struct DictationKeys {
    talk: Arc<PushToTalk>,
    keys: Sender<bool>,
}

impl DictationKeys {
    pub(crate) fn start<R: Runtime>(
        app: &AppHandle<R>,
        overlay: Arc<OverlayController>,
    ) -> io::Result<Self> {
        let host = Host { app: app.clone() };
        let talk = Arc::new(PushToTalk::new(Arc::new(host), overlay));
        let (keys, presses) = mpsc::channel();
        let worker = Arc::clone(&talk);
        thread::Builder::new()
            .name("dictation-keys".into())
            .spawn(move || {
                for pressed in presses {
                    worker.key(pressed);
                }
            })?;
        Ok(Self { talk, keys })
    }

    pub(crate) fn key(&self, pressed: bool) {
        self.talk.announce(pressed);
        log_failure(self.keys.send(pressed), "pass on the dictation key");
    }
}

struct Host<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> DictationHost for Host<R> {
    fn dictation(&self) -> Arc<DictationService> {
        self.app.state::<Voice>().dictation()
    }

    fn follow_level(&self, level: Box<dyn Fn() -> Option<f32> + Send>) {
        overlay::pump_levels(&self.app, level);
    }

    fn spawn(&self, task: Box<dyn FnOnce() + Send>) {
        support::spawn("dictation-finish", task);
    }

    fn pressed(&self, result: &Result<PressOutcome, DictationError>) {
        match result {
            Ok(outcome) => tracing::info!(%outcome, "dictation key down"),
            Err(error) => tracing::warn!(%error, "dictation could not start"),
        }
    }

    fn finished(&self, result: &Result<DictationOutcome, DictationError>) {
        match result {
            Ok(outcome) => tracing::info!(%outcome, "dictation finished"),
            Err(error) => tracing::warn!(%error, "dictation failed"),
        }
    }
}
