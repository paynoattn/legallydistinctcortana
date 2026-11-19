//! Tauri shell for the assistant: builds the windows from `tauri.conf.json`, wires the
//! feature crates' adapters into their use cases, and runs the event loop. Feature logic
//! does not live here; it lives in the feature crates.

mod connection;
mod dictation;
mod overlay;
mod read_aloud;
mod settings;
mod shortcuts;
mod support;
mod tray;
mod voice;
mod windows;

use std::env;
use std::error::Error;

use tauri::{App, AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;

use crate::connection::Connection;
use crate::dictation::DictationKeys;
use crate::overlay::Overlay;
use crate::settings::SettingsState;
use crate::shortcuts::Shortcuts;
use crate::voice::Voice;
use crate::windows::BACKGROUND_FLAG;

/// Builds and runs the Tauri application until it exits.
///
/// # Errors
///
/// Returns the Tauri error if the runtime fails to start or exits abnormally.
pub fn run() -> Result<(), tauri::Error> {
    tracing_subscriber::fmt().init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            windows::show_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_FLAG]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            read_aloud::read_aloud,
            read_aloud::current_selection,
            read_aloud::list_voices,
            read_aloud::stop_reading,
            read_aloud::dismiss_read_aloud,
            overlay::overlay_fix,
            overlay::overlay_dismiss,
            settings::settings_get,
            settings::settings_default,
            settings::settings_save,
            settings::shortcuts_suspend,
            settings::shortcuts_resume,
            connection::connection_status,
            connection::check_connection
        ])
        .setup(setup)
        .on_window_event(|window, event| {
            read_aloud::on_window_event(window, event);
            windows::on_window_event(window, event);
        })
        .run(tauri::generate_context!())
}

fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let settings_state = SettingsState::open(app)?;
    let settings = settings_state.current();
    app.manage(settings_state);
    app.manage(Voice::new(&settings)?);
    let overlay = Overlay::new(app.handle(), settings.show_overlay, settings.reduce_motion);
    app.manage(DictationKeys::start(app.handle(), overlay.controller())?);
    app.manage(overlay);
    read_aloud::setup(app);
    let (connection, poked) = Connection::new(&settings.voice_host);
    tray::setup(app, &connection.status())?;
    app.manage(connection);
    Connection::watch(app.handle(), poked);
    app.manage(Shortcuts::default());
    put_in_effect(app.handle());
    windows::create_deferred(app.handle())?;
    if !env::args().any(|argument| argument == BACKGROUND_FLAG) {
        windows::show_main(app.handle());
    }
    Ok(())
}

fn put_in_effect(app: &AppHandle) {
    shortcuts::log_refused(app.state::<SettingsState>().put_in_effect(app));
}
