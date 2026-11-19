//! Wiring for the corner indicator: the `overlay` window, its events, and its two
//! commands. What it shows is decided by `speech_application::OverlayController`.

use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use platform_ports::{Point, Rect, Size, corner_origin};
use serde::Serialize;
use speech_application::{
    Delay, Fix, OverlayController, OverlayEvent, OverlayState, OverlaySurface,
};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Runtime, State, WebviewWindow,
};

use crate::support::{self, log_failure};

pub(crate) const WINDOW: &str = "overlay";
const STATE_EVENT: &str = "overlay:state";
const LEVEL_EVENT: &str = "overlay:level";
const MARGIN: f64 = 16.0;
const FADE_OUT: Duration = Duration::from_millis(650);
const LEVEL_INTERVAL: Duration = Duration::from_millis(33);
const SOUND_SETTINGS: &str = "ms-settings:sound";

pub(crate) struct Overlay {
    controller: Arc<OverlayController>,
    preferences: Arc<Preferences>,
}

#[derive(Default)]
pub(crate) struct Preferences {
    enabled: AtomicBool,
    reduce_motion: AtomicBool,
}

impl Preferences {
    pub(crate) fn set(&self, enabled: bool, reduce_motion: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.reduce_motion.store(reduce_motion, Ordering::Relaxed);
    }

    fn shows(&self, state: &OverlayState) -> bool {
        self.enabled.load(Ordering::Relaxed) && state.is_visible()
    }
}

impl Overlay {
    pub(crate) fn new<R: Runtime>(app: &AppHandle<R>, enabled: bool, reduce_motion: bool) -> Self {
        let preferences = Arc::new(Preferences::default());
        preferences.set(enabled, reduce_motion);
        let surface = TauriSurface {
            app: app.clone(),
            preferences: Arc::clone(&preferences),
            shown: Arc::new(AtomicU64::new(0)),
        };
        Self {
            controller: OverlayController::new(Arc::new(surface), Arc::new(ThreadDelay)),
            preferences,
        }
    }

    pub(crate) fn handle(&self, event: OverlayEvent) {
        self.controller.handle(event);
    }

    pub(crate) fn redraw(&self) {
        self.controller.redraw();
    }

    pub(crate) fn controller(&self) -> Arc<OverlayController> {
        Arc::clone(&self.controller)
    }

    pub(crate) fn preferences(&self) -> &Preferences {
        &self.preferences
    }
}

/// Sends the input level to the indicator about 30 times a second while `listening` holds.
pub(crate) fn pump_levels<R: Runtime>(
    app: &AppHandle<R>,
    level: impl Fn() -> Option<f32> + Send + 'static,
) {
    let app = app.clone();
    support::spawn("overlay-level", move || {
        while let Some(level) = level() {
            drop(app.emit_to(WINDOW, LEVEL_EVENT, LevelPayload { level }));
            thread::sleep(LEVEL_INTERVAL);
        }
    });
}

#[derive(Debug, Clone, Copy, Serialize)]
struct LevelPayload {
    level: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatePayload {
    state: StateDto,
    reduce_motion: bool,
}

#[derive(Debug, Clone, Serialize)]
struct StateDto {
    kind: &'static str,
    reason: Option<String>,
    fix: Option<&'static str>,
}

impl From<&OverlayState> for StateDto {
    fn from(state: &OverlayState) -> Self {
        let error = state.error();
        Self {
            kind: state.id(),
            reason: error.map(|failure| failure.reason.clone()),
            fix: error.and_then(|failure| failure.fix).map(Fix::id),
        }
    }
}

struct TauriSurface<R: Runtime> {
    app: AppHandle<R>,
    preferences: Arc<Preferences>,
    shown: Arc<AtomicU64>,
}

impl<R: Runtime> OverlaySurface for TauriSurface<R> {
    fn show(&self, state: &OverlayState) {
        let visible = self.preferences.shows(state);
        let clicks = state.takes_clicks();
        let generation = self.shown.fetch_add(1, Ordering::SeqCst) + 1;
        let payload = StatePayload {
            state: StateDto::from(state),
            reduce_motion: self.preferences.reduce_motion.load(Ordering::Relaxed),
        };
        log_failure(
            self.app.emit_to(WINDOW, STATE_EVENT, payload),
            "update the indicator",
        );
        if visible {
            let app = self.app.clone();
            log_failure(
                self.app.run_on_main_thread(move || reveal(&app, clicks)),
                "show the indicator",
            );
        } else {
            self.hide_after_fade(generation);
        }
    }
}

impl<R: Runtime> TauriSurface<R> {
    fn hide_after_fade(&self, generation: u64) {
        let app = self.app.clone();
        let shown = Arc::clone(&self.shown);
        ThreadDelay.after(
            FADE_OUT,
            Box::new(move || {
                let target = app.clone();
                log_failure(
                    app.run_on_main_thread(move || {
                        if shown.load(Ordering::SeqCst) == generation {
                            conceal(&target);
                        }
                    }),
                    "hide the indicator",
                );
            }),
        );
    }
}

struct ThreadDelay;

impl Delay for ThreadDelay {
    fn after(&self, wait: Duration, task: Box<dyn FnOnce() + Send>) {
        support::spawn("overlay-timer", move || {
            thread::sleep(wait);
            task();
        });
    }
}

fn reveal<R: Runtime>(app: &AppHandle<R>, clicks: bool) {
    if let Some(window) = overlay_window(app) {
        reveal_window(app, &window, clicks);
    }
}

fn reveal_window<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, clicks: bool) {
    log_failure(
        window.set_ignore_cursor_events(!clicks),
        "make the indicator click-through",
    );
    if !window.is_visible().unwrap_or(false) {
        log_failure(place(app, window), "place the indicator");
        log_failure(window.show(), "show the indicator");
    }
}

fn conceal<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = overlay_window(app) {
        log_failure(window.hide(), "hide the indicator");
    }
}

fn overlay_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    let window = app.get_webview_window(WINDOW);
    if window.is_none() {
        tracing::warn!("the overlay window is missing from tauri.conf.json");
    }
    window
}

fn place<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    let Some(monitor) = app.monitor_from_point(cursor.x, cursor.y)? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let size = window
        .inner_size()?
        .to_logical::<f64>(window.scale_factor()?);
    let size = size.to_physical::<u32>(scale);
    let margin = LogicalSize::new(MARGIN, MARGIN)
        .to_physical::<u32>(scale)
        .width;
    let area = monitor.work_area();
    let origin = corner_origin(
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
        margin,
    );
    window.set_position(PhysicalPosition::new(origin.x, origin.y))
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn overlay_fix(
    app: AppHandle,
    window: WebviewWindow,
    overlay: State<'_, Overlay>,
) -> Result<(), String> {
    only_from(&window)?;
    if let Some(fix) = overlay.controller.dismiss() {
        carry_out(&app, fix);
    }
    Ok(())
}

fn carry_out<R: Runtime>(app: &AppHandle<R>, fix: Fix) {
    match fix {
        Fix::SoundSettings => open_sound_settings(),
        Fix::Settings => crate::windows::show_settings(app),
    }
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn overlay_dismiss(
    window: WebviewWindow,
    overlay: State<'_, Overlay>,
) -> Result<(), String> {
    only_from(&window)?;
    overlay.handle(OverlayEvent::Dismissed);
    Ok(())
}

fn only_from(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == WINDOW {
        Ok(())
    } else {
        Err("only the indicator can do this".into())
    }
}

fn open_sound_settings() {
    let opened = Command::new("explorer").arg(SOUND_SETTINGS).spawn();
    log_failure(opened.map(drop), "open the sound settings");
}
