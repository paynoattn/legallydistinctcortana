//! Wiring for the settings: the file store, the settings window's commands, and applying a
//! saved change (shortcuts, voice server, indicator, start at sign-in) without a restart.

use std::env;
use std::error::Error;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use settings_application::{
    FieldError, SaveError, Settings, SettingsEffects, SettingsInput, SettingsService, SettingsStore,
};
use settings_infrastructure::JsonFileStore;
use tauri::{App, AppHandle, Manager, Runtime, State, WebviewWindow};
use tauri_plugin_autostart::{AutoLaunchManager, Error as AutostartError, ManagerExt};

use crate::connection::Connection;
use crate::overlay::Overlay;
use crate::read_aloud;
use crate::shortcuts::Shortcuts;
use crate::support::log_failure;
use crate::voice::Voice;
use crate::windows::SETTINGS;

const FILE: &str = "settings.json";

/// Debug builds leave the sign-in items alone, so a development build never starts at
/// sign-in. Test the setting with an installed build.
const CHANGES_SIGN_IN: bool = cfg!(not(debug_assertions));

pub(crate) struct SettingsState(Arc<SettingsService>);

impl SettingsState {
    pub(crate) fn open(app: &App) -> Result<Self, Box<dyn Error>> {
        let store = JsonFileStore::new(app.path().app_config_dir()?.join(FILE));
        tracing::info!(path = %store.path().display(), "settings file");
        let store: Arc<dyn SettingsStore> = Arc::new(store);
        let (service, warnings) =
            SettingsService::open(store, Settings::defaults(|key| env::var(key).ok()));
        for warning in warnings {
            tracing::warn!(%warning, "settings");
        }
        Ok(Self(Arc::new(service)))
    }

    pub(crate) fn current(&self) -> Settings {
        self.0.current()
    }

    /// Puts the stored settings in effect at startup. Returns the shortcuts that could not
    /// be registered.
    pub(crate) fn put_in_effect<R: Runtime>(&self, app: &AppHandle<R>) -> Vec<FieldError> {
        self.0.put_in_effect(&Effects(app))
    }
}

/// The running app, as the settings service changes it.
struct Effects<'a, R: Runtime>(&'a AppHandle<R>);

impl<R: Runtime> SettingsEffects for Effects<'_, R> {
    fn indicator(&self, settings: &Settings) {
        let overlay = self.0.state::<Overlay>();
        overlay
            .preferences()
            .set(settings.show_overlay, settings.reduce_motion);
        overlay.redraw();
    }

    fn voice_server(&self, settings: &Settings) {
        log_failure(
            self.0.state::<Voice>().rebuild(settings),
            "switch the voice server",
        );
        read_aloud::refresh_voices(self.0);
        self.0.state::<Connection>().check_soon();
    }

    fn start_at_sign_in(&self, enabled: bool) -> Result<(), String> {
        if !CHANGES_SIGN_IN {
            return Ok(());
        }
        sign_in_entry(&self.0.autolaunch(), enabled).map_err(|error| {
            tracing::warn!(%error, enabled, "could not change start at sign-in");
            "Windows didn't let the app change its sign-in items.".to_owned()
        })
    }

    fn shortcuts(&self, settings: &Settings) -> Vec<FieldError> {
        self.0.state::<Shortcuts>().refresh(self.0, settings)
    }
}

fn sign_in_entry(autolaunch: &AutoLaunchManager, enabled: bool) -> Result<(), AutostartError> {
    if autolaunch.is_enabled()? == enabled {
        return Ok(());
    }
    toggle_sign_in_entry(autolaunch, enabled)
}

fn toggle_sign_in_entry(
    autolaunch: &AutoLaunchManager,
    enabled: bool,
) -> Result<(), AutostartError> {
    tracing::info!(enabled, "start at sign-in");
    if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    }
}

/// The settings as the settings window sees them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsPayload {
    dictate_shortcut: String,
    read_aloud_shortcut: String,
    voice_host: String,
    stt_port: i64,
    tts_port: i64,
    stt_language: String,
    reduce_motion: bool,
    show_overlay: bool,
    start_at_sign_in: bool,
}

impl From<&Settings> for SettingsPayload {
    fn from(settings: &Settings) -> Self {
        Self {
            dictate_shortcut: settings.dictate_shortcut.to_string(),
            read_aloud_shortcut: settings.read_aloud_shortcut.to_string(),
            voice_host: settings.voice_host.clone(),
            stt_port: i64::from(settings.stt_port),
            tts_port: i64::from(settings.tts_port),
            stt_language: settings.stt_language.clone(),
            reduce_motion: settings.reduce_motion,
            show_overlay: settings.show_overlay,
            start_at_sign_in: settings.start_at_sign_in,
        }
    }
}

impl From<SettingsPayload> for SettingsInput {
    fn from(payload: SettingsPayload) -> Self {
        Self {
            dictate_shortcut: Some(payload.dictate_shortcut),
            read_aloud_shortcut: Some(payload.read_aloud_shortcut),
            voice_host: Some(payload.voice_host),
            stt_port: Some(payload.stt_port),
            tts_port: Some(payload.tts_port),
            stt_language: Some(payload.stt_language),
            reduce_motion: Some(payload.reduce_motion),
            show_overlay: Some(payload.show_overlay),
            start_at_sign_in: Some(payload.start_at_sign_in),
        }
    }
}

/// Why a save was refused, field by field.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErrorsPayload {
    errors: Vec<FieldErrorPayload>,
}

#[derive(Debug, Clone, Serialize)]
struct FieldErrorPayload {
    field: &'static str,
    message: String,
}

impl ErrorsPayload {
    fn fields(errors: Vec<FieldError>) -> Self {
        Self {
            errors: errors
                .into_iter()
                .map(|error| FieldErrorPayload {
                    field: error.field.id(),
                    message: error.message,
                })
                .collect(),
        }
    }

    fn general(message: String) -> Self {
        Self {
            errors: vec![FieldErrorPayload {
                field: "general",
                message,
            }],
        }
    }
}

fn only_from(window: &WebviewWindow) -> Result<(), ErrorsPayload> {
    if window.label() == SETTINGS {
        Ok(())
    } else {
        Err(ErrorsPayload::general(
            "Only the settings window can change settings.".into(),
        ))
    }
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn settings_get(
    window: WebviewWindow,
    state: State<'_, SettingsState>,
) -> Result<SettingsPayload, ErrorsPayload> {
    only_from(&window)?;
    Ok(SettingsPayload::from(&state.current()))
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn settings_default(
    window: WebviewWindow,
    state: State<'_, SettingsState>,
) -> Result<SettingsPayload, ErrorsPayload> {
    only_from(&window)?;
    Ok(SettingsPayload::from(&state.0.defaults()))
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn settings_save(
    window: WebviewWindow,
    state: State<'_, SettingsState>,
    settings: SettingsPayload,
) -> Result<SettingsPayload, ErrorsPayload> {
    only_from(&window)?;
    state
        .0
        .save(&settings.into(), &Effects(window.app_handle()))
        .map(|saved| SettingsPayload::from(&saved))
        .map_err(save_errors)
}

fn save_errors(error: SaveError) -> ErrorsPayload {
    match error {
        SaveError::Invalid(errors) | SaveError::Refused(errors) => ErrorsPayload::fields(errors),
        SaveError::Store(error) => ErrorsPayload::general(format!("Couldn't save: {error}")),
    }
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn shortcuts_suspend(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<(), ErrorsPayload> {
    only_from(&window)?;
    app.state::<Shortcuts>().set_suspended(&app, true);
    Ok(())
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]
pub(crate) fn shortcuts_resume(app: AppHandle, window: WebviewWindow) -> Result<(), ErrorsPayload> {
    only_from(&window)?;
    app.state::<Shortcuts>().set_suspended(&app, false);
    Ok(())
}
