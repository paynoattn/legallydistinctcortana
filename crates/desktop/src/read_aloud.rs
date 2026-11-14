//! Wiring for "read selected text aloud": the global shortcut, the context window next to
//! the cursor, and the commands the window calls. The behavior lives in `speech-application`.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};
use std::{env, thread};

use kernel::{Mood, UnknownMood};
use platform_desktop::{ArboardClipboard, ClipboardSelectionReader, EnigoKeyboard};
use platform_ports::{Point, Rect, Size, popup_origin};
use serde::Serialize;
use speech_application::{
    ReadAloudError, ReadAloudService, ReadOutcome, SelectionPreview, ShortcutOutcome,
};
use speech_infrastructure::{SpeakerOutput, WyomingConfig, WyomingSynthesizer};
use tauri::{
    App, AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State, WebviewWindow, Window,
    WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const POPUP: &str = "read-aloud";
const SELECTION_EVENT: &str = "read-aloud:selection";
const STATUS_EVENT: &str = "read-aloud:status";
const SHORTCUT_VARIABLE: &str = "READ_ALOUD_SHORTCUT";
const DEFAULT_SHORTCUT: &str = "ctrl+alt+r";

pub(crate) struct ReadAloud {
    service: Arc<ReadAloudService>,
    last_selection: Mutex<Option<SelectionPayload>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SelectionPayload {
    Ready { preview: String, char_count: usize },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StatusPayload {
    Reading { mood: &'static str },
    Finished,
    Stopped,
    Failed { message: String },
}

impl From<Result<SelectionPreview, ReadAloudError>> for SelectionPayload {
    fn from(result: Result<SelectionPreview, ReadAloudError>) -> Self {
        match result {
            Ok(preview) => Self::Ready {
                preview: preview.text,
                char_count: preview.char_count,
            },
            Err(error) => Self::Failed {
                message: error.to_string(),
            },
        }
    }
}

impl From<ReadOutcome> for StatusPayload {
    fn from(outcome: ReadOutcome) -> Self {
        match outcome {
            ReadOutcome::Finished => Self::Finished,
            ReadOutcome::Stopped => Self::Stopped,
        }
    }
}

impl From<Result<ReadOutcome, ReadAloudError>> for StatusPayload {
    fn from(result: Result<ReadOutcome, ReadAloudError>) -> Self {
        result.map_or_else(
            |error| Self::Failed {
                message: error.to_string(),
            },
            Self::from,
        )
    }
}

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let config = WyomingConfig::from_lookup(|key| env::var(key).ok())?;
    tracing::info!(server = %config.address(), "read aloud uses this Wyoming voice server");
    let service = Arc::new(ReadAloudService::new(
        Arc::new(ClipboardSelectionReader::new(
            ArboardClipboard,
            EnigoKeyboard,
        )),
        Arc::new(WyomingSynthesizer::new(config)),
        Arc::new(SpeakerOutput::default()),
    ));
    app.manage(ReadAloud {
        service: Arc::clone(&service),
        last_selection: Mutex::new(None),
    });
    let shortcut: Shortcut = shortcut_setting().parse()?;
    tracing::info!(%shortcut, "read aloud shortcut registered");
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state() == ShortcutState::Released {
                on_shortcut(app, &service);
            }
        })?;
    Ok(())
}

fn shortcut_setting() -> String {
    env::var(SHORTCUT_VARIABLE)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_owned())
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, service: &Arc<ReadAloudService>) {
    let app = app.clone();
    let service = Arc::clone(service);
    thread::spawn(move || {
        if let ShortcutOutcome::Captured(selection) = service.toggle() {
            let payload = SelectionPayload::from(selection);
            remember(&app, payload.clone());
            show_popup(&app);
            emit(&app, SELECTION_EVENT, payload);
        }
    });
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn read_aloud(
    app: AppHandle,
    state: State<'_, ReadAloud>,
    mood: String,
) -> Result<(), String> {
    let mood: Mood = mood
        .parse()
        .map_err(|error: UnknownMood| error.to_string())?;
    let ticket = state
        .service
        .begin(mood)
        .map_err(|error| error.to_string())?;
    emit(
        &app,
        STATUS_EVENT,
        StatusPayload::Reading { mood: mood.id() },
    );
    let service = Arc::clone(&state.service);
    thread::spawn(move || {
        if let Some(result) = service.finish(ticket) {
            report_outcome(&app, result);
        }
    });
    Ok(())
}

fn report_outcome<R: Runtime>(app: &AppHandle<R>, result: Result<ReadOutcome, ReadAloudError>) {
    let status = StatusPayload::from(result);
    let finished = matches!(status, StatusPayload::Finished);
    emit(app, STATUS_EVENT, status);
    if finished {
        hide_popup(app);
    }
}

fn remember<R: Runtime>(app: &AppHandle<R>, payload: SelectionPayload) {
    let state = app.state::<ReadAloud>();
    *state
        .last_selection
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(payload);
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn current_selection(state: State<'_, ReadAloud>) -> Option<SelectionPayload> {
    state
        .last_selection
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn stop_reading(state: State<'_, ReadAloud>) {
    state.service.stop();
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn dismiss_read_aloud(app: AppHandle) {
    hide_popup(&app);
}

pub(crate) fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() == POPUP {
        hide_when_blurred(window, event);
    }
}

fn hide_when_blurred<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if matches!(event, WindowEvent::Focused(false)) {
        log_failure(window.hide(), "hide the read-aloud window");
    }
}

fn emit<R: Runtime>(app: &AppHandle<R>, event: &str, payload: impl Serialize + Clone) {
    log_failure(
        app.emit_to(POPUP, event, payload),
        "update the read-aloud window",
    );
}

fn show_popup<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP) {
        log_failure(place_and_show(app, &window), "show the read-aloud window");
    } else {
        tracing::warn!("the read-aloud window is missing from tauri.conf.json");
    }
}

fn hide_popup<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP) {
        log_failure(window.hide(), "hide the read-aloud window");
    }
}

fn place_and_show<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    if let Some(monitor) = app.monitor_from_point(cursor.x, cursor.y)? {
        let size = window.outer_size()?;
        let area = monitor.work_area();
        let cursor = cursor.cast::<i32>();
        let origin = popup_origin(
            Point {
                x: cursor.x,
                y: cursor.y,
            },
            Size {
                width: size.width,
                height: size.height,
            },
            Rect {
                origin: Point {
                    x: area.position.x,
                    y: area.position.y,
                },
                size: Size {
                    width: area.size.width,
                    height: area.size.height,
                },
            },
        );
        window.set_position(PhysicalPosition::new(origin.x, origin.y))?;
    }
    window.show()?;
    window.set_focus()
}

fn log_failure(result: tauri::Result<()>, action: &str) {
    if let Err(error) = result {
        tracing::warn!(%error, "could not {action}");
    }
}
