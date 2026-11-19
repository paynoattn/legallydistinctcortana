//! Wiring for "read selected text aloud": what the shortcut does, the context window next to
//! the cursor, and the commands the window calls. The behavior lives in `speech-application`;
//! the service itself comes from [`Voice`], so it follows the voice server setting.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;

use kernel::{Mood, UnknownMood};
use platform_ports::{Point, Rect, Size, popup_origin};
use serde::{Deserialize, Serialize};
use speech_application::{
    OverlayEvent, ReadAloudError, ReadOutcome, SelectionPreview, ShortcutOutcome,
    Voice as VoiceInfo, VoiceChoice,
};
use tauri::{
    App, AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State, WebviewWindow, Window,
    WindowEvent,
};

use crate::overlay::Overlay;
use crate::voice::Voice;

const POPUP: &str = "read-aloud";
const SELECTION_EVENT: &str = "read-aloud:selection";
const STATUS_EVENT: &str = "read-aloud:status";

pub(crate) struct ReadAloud {
    last_selection: Mutex<Option<SelectionPayload>>,
    latest_reading: Mutex<u64>,
}

impl ReadAloud {
    /// The number of the latest reading. Readings are numbered, and announced or reported
    /// while this is held, so a reading stopped by a voice server change cannot overwrite the
    /// status of one started since.
    fn latest_reading(&self) -> MutexGuard<'_, u64> {
        self.latest_reading
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
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

/// What the read-aloud window asks for when a mood is chosen.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadRequest {
    mood: String,
    voice: Option<String>,
    speaker: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoicePayload {
    name: String,
    description: String,
    languages: Vec<String>,
    speakers: Vec<String>,
}

impl From<VoiceInfo> for VoicePayload {
    fn from(voice: VoiceInfo) -> Self {
        Self {
            name: voice.name,
            description: voice.description,
            languages: voice.languages,
            speakers: voice.speakers,
        }
    }
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

pub(crate) fn setup(app: &App) {
    app.manage(ReadAloud {
        last_selection: Mutex::new(None),
        latest_reading: Mutex::new(0),
    });
}

/// The read-aloud shortcut was released: capture the selection and show the window, or stop
/// a reading in progress.
pub(crate) fn on_shortcut<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    thread::spawn(move || {
        let service = app.state::<Voice>().read_aloud();
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
    voice: State<'_, Voice>,
    request: ReadRequest,
) -> Result<(), String> {
    let mood: Mood = request
        .mood
        .parse()
        .map_err(|error: UnknownMood| error.to_string())?;
    let voice_choice = VoiceChoice::parse(request.voice.as_deref(), request.speaker.as_deref());
    let service = voice.read_aloud();
    let ticket = service
        .begin(mood, voice_choice)
        .map_err(|error| error.to_string())?;
    let reading = announce_reading(&app, mood);
    thread::spawn(move || {
        if let Some(result) = service.finish(ticket) {
            report_if_latest(&app, reading, result);
        }
    });
    Ok(())
}

fn announce_reading<R: Runtime>(app: &AppHandle<R>, mood: Mood) -> u64 {
    let state = app.state::<ReadAloud>();
    let mut latest = state.latest_reading();
    *latest = latest.wrapping_add(1);
    app.state::<Overlay>().handle(OverlayEvent::SpeakingStarted);
    emit(
        app,
        STATUS_EVENT,
        StatusPayload::Reading { mood: mood.id() },
    );
    *latest
}

fn report_if_latest<R: Runtime>(
    app: &AppHandle<R>,
    reading: u64,
    result: Result<ReadOutcome, ReadAloudError>,
) {
    if publish_if_latest(app, reading, result) {
        hide_popup(app);
    }
}

/// Publishes how `reading` ended unless a newer one has started. True when it finished.
fn publish_if_latest<R: Runtime>(
    app: &AppHandle<R>,
    reading: u64,
    result: Result<ReadOutcome, ReadAloudError>,
) -> bool {
    let state = app.state::<ReadAloud>();
    let latest = state.latest_reading();
    *latest == reading && publish_outcome(app, result)
}

/// Reloads the read-aloud window so it lists the voices of the voice server now in use.
pub(crate) fn refresh_voices<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP) {
        log_failure(window.reload(), "refresh the voice list");
    }
}

fn publish_outcome<R: Runtime>(
    app: &AppHandle<R>,
    result: Result<ReadOutcome, ReadAloudError>,
) -> bool {
    app.state::<Overlay>().handle(OverlayEvent::SpeakingEnded);
    let status = StatusPayload::from(result);
    let finished = matches!(status, StatusPayload::Finished);
    emit(app, STATUS_EVENT, status);
    finished
}

fn remember<R: Runtime>(app: &AppHandle<R>, payload: SelectionPayload) {
    let state = app.state::<ReadAloud>();
    *state
        .last_selection
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(payload);
}

#[tauri::command(async)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn list_voices(voice: State<'_, Voice>) -> Result<Vec<VoicePayload>, String> {
    voice
        .read_aloud()
        .voices()
        .map(|voices| voices.into_iter().map(VoicePayload::from).collect())
        .map_err(|error| error.to_string())
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
pub(crate) fn stop_reading(voice: State<'_, Voice>) {
    voice.read_aloud().stop();
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
