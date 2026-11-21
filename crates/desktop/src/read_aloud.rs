//! Wiring for "read selected text aloud": what the shortcut does, the reader window centred on
//! the monitor under the cursor, and the commands the window calls. The behavior lives in `speech-application`;
//! the service itself comes from [`Voice`], so it follows the voice server setting.

use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use kernel::{Mood, UnknownMood};
use platform_ports::{Point, Rect, Size, centred_origin};
use serde::{Deserialize, Serialize};
use speech_application::{
    OverlayEvent, ReadAloudError, ReadOutcome, Readings, SelectionPreview, Sentence,
    SentenceTiming, Voice as VoiceInfo, VoiceChoice,
};
use tauri::{
    App, AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime, State,
    WebviewWindow,
};

use crate::overlay::Overlay;
use crate::voice::Voice;

const POPUP: &str = "read-aloud";
const SELECTION_EVENT: &str = "read-aloud:selection";
const STATUS_EVENT: &str = "read-aloud:status";
const NORMAL_SPEED: u32 = 1_000;
const SENTENCE_EVENT: &str = "read-aloud:sentence";
const READER_WIDTH: f64 = 860.0;
const READER_HEIGHT: f64 = 660.0;

pub(crate) struct ReadAloud {
    last_selection: Mutex<Option<SelectionPayload>>,
    readings: Readings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SelectionPayload {
    Ready {
        preview: String,
        char_count: usize,
        sentences: Vec<SentencePayload>,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SentencePayload {
    text: String,
    starts_paragraph: bool,
}

impl From<Sentence> for SentencePayload {
    fn from(sentence: Sentence) -> Self {
        Self {
            text: sentence.text,
            starts_paragraph: sentence.starts_paragraph,
        }
    }
}

/// When a queued sentence will be heard, for the reader's word timing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SentencePlayback {
    reading: u64,
    index: usize,
    starts_in_ms: u64,
    speech_ms: u64,
}

impl SentencePlayback {
    fn new(reading: u64, timing: SentenceTiming) -> Self {
        Self {
            reading,
            index: timing.index,
            starts_in_ms: millis(timing.starts_in),
            speech_ms: millis(timing.speech),
        }
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
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

/// What the reader asks for: a mood, a voice, and the sentence to start from (the first when
/// absent).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadRequest {
    mood: String,
    voice: Option<String>,
    speaker: Option<String>,
    from_sentence: Option<usize>,
    /// Reading speed in thousandths (1000 = normal), pitch kept.
    speed: Option<u32>,
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
                sentences: preview
                    .sentences
                    .into_iter()
                    .map(SentencePayload::from)
                    .collect(),
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
        readings: Readings::default(),
    });
}

/// The read-aloud shortcut was released: stop any reading, capture the selection, and show
/// it in the reader, which starts reading it.
pub(crate) fn on_shortcut<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    thread::spawn(move || {
        let service = app.state::<Voice>().read_aloud();
        service.stop();
        let payload = SelectionPayload::from(service.capture_selection());
        remember(&app, payload.clone());
        show_popup(&app);
        emit(&app, SELECTION_EVENT, payload);
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
    let from = request.from_sentence.unwrap_or(0);
    let speed = request.speed.unwrap_or(NORMAL_SPEED);
    let ticket = service
        .begin(mood, voice_choice)
        .map(|ticket| ticket.starting_at(from).at_tempo(speed))
        .map_err(|error| error.to_string())?;
    let reading = announce_reading(&app, mood);
    thread::spawn(move || {
        let report = |timing| report_sentence(&app, reading, timing);
        if let Some(result) = service.finish(ticket, report) {
            publish_if_latest(&app, reading, result);
        }
    });
    Ok(())
}

fn report_sentence<R: Runtime>(app: &AppHandle<R>, reading: u64, timing: SentenceTiming) {
    app.state::<ReadAloud>()
        .readings
        .report_if_latest(reading, || {
            emit(app, SENTENCE_EVENT, SentencePlayback::new(reading, timing));
        });
}

fn announce_reading<R: Runtime>(app: &AppHandle<R>, mood: Mood) -> u64 {
    app.state::<ReadAloud>().readings.begin(|| {
        app.state::<Overlay>().handle(OverlayEvent::SpeakingStarted);
        emit(
            app,
            STATUS_EVENT,
            StatusPayload::Reading { mood: mood.id() },
        );
    })
}

/// Publishes how `reading` ended unless a newer one has started.
fn publish_if_latest<R: Runtime>(
    app: &AppHandle<R>,
    reading: u64,
    result: Result<ReadOutcome, ReadAloudError>,
) {
    app.state::<ReadAloud>()
        .readings
        .report_if_latest(reading, || publish_outcome(app, result));
}

/// Reloads the read-aloud window so it lists the voices of the voice server now in use.
pub(crate) fn refresh_voices<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP) {
        log_failure(window.reload(), "refresh the voice list");
    }
}

fn publish_outcome<R: Runtime>(app: &AppHandle<R>, result: Result<ReadOutcome, ReadAloudError>) {
    app.state::<Overlay>().handle(OverlayEvent::SpeakingEnded);
    emit(app, STATUS_EVENT, StatusPayload::from(result));
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

/// Centres the reader, at its logical size, on the monitor under the cursor, then shows and
/// focuses it.
fn place_and_show<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    if let Some(monitor) = app.monitor_from_point(cursor.x, cursor.y)? {
        let size: PhysicalSize<u32> =
            LogicalSize::new(READER_WIDTH, READER_HEIGHT).to_physical(monitor.scale_factor());
        let area = monitor.work_area();
        let origin = centred_origin(
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
        window.set_size(size)?;
    }
    window.show()?;
    window.set_focus()
}

fn log_failure(result: tauri::Result<()>, action: &str) {
    if let Err(error) = result {
        tracing::warn!(%error, "could not {action}");
    }
}
