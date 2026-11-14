//! Wiring for push-to-talk dictation: hold the shortcut (F9 by default) to record, release
//! it to type what was said into the focused app. The behavior lives in `speech-application`.

use std::error::Error;
use std::sync::Arc;
use std::{env, thread};

use platform_desktop::{ArboardClipboard, ClipboardTextInserter, EnigoKeyboard};
use speech_application::{DictationError, DictationOutcome, DictationService, PressOutcome};
use speech_infrastructure::{MicrophoneInput, WyomingSttConfig, WyomingTranscriber};
use tauri::App;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const SHORTCUT_VARIABLE: &str = "DICTATE_SHORTCUT";
const DEFAULT_SHORTCUT: &str = "F9";

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let config = WyomingSttConfig::from_lookup(|key| env::var(key).ok())?;
    tracing::info!(server = %config.address(), "dictation uses this Wyoming speech recognition server");
    let service = Arc::new(DictationService::new(
        Arc::new(MicrophoneInput::default()),
        Arc::new(WyomingTranscriber::new(config)),
        Arc::new(ClipboardTextInserter::new(ArboardClipboard, EnigoKeyboard)),
    ));
    let shortcut: Shortcut = shortcut_setting().parse()?;
    let registered = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_, _, event| {
            on_key(&service, event.state() == ShortcutState::Pressed);
        });
    log_registration(registered.map(|()| shortcut));
    Ok(())
}

fn log_registration(registered: Result<Shortcut, tauri_plugin_global_shortcut::Error>) {
    match registered {
        Ok(shortcut) => tracing::info!(%shortcut, "dictation shortcut registered; hold it to talk"),
        Err(error) => {
            tracing::warn!(%error, "dictation is off: its shortcut could not be registered");
        }
    }
}

fn shortcut_setting() -> String {
    env::var(SHORTCUT_VARIABLE)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_owned())
}

fn on_key(service: &Arc<DictationService>, pressed: bool) {
    if pressed {
        log_press(service.press());
    } else {
        release(service);
    }
}

fn log_press(result: Result<PressOutcome, DictationError>) {
    match result {
        Ok(outcome) => tracing::info!(%outcome, "dictation key down"),
        Err(error) => tracing::warn!(%error, "dictation could not start"),
    }
}

fn release(service: &Arc<DictationService>) {
    if let Some(utterance) = service.release() {
        let service = Arc::clone(service);
        thread::spawn(move || log_outcome(service.finish(utterance)));
    }
}

fn log_outcome(result: Result<DictationOutcome, DictationError>) {
    match result {
        Ok(outcome) => tracing::info!(%outcome, "dictation finished"),
        Err(error) => tracing::warn!(%error, "dictation failed"),
    }
}
