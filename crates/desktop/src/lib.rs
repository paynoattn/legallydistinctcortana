//! Tauri shell for the assistant: builds the windows from `tauri.conf.json`, wires the
//! feature crates' adapters into their use cases, and runs the event loop. Feature logic
//! does not live here; it lives in the feature crates.

mod read_aloud;

/// Builds and runs the Tauri application until it exits.
///
/// # Errors
///
/// Returns the Tauri error if the runtime fails to start or exits abnormally.
pub fn run() -> Result<(), tauri::Error> {
    tracing_subscriber::fmt().init();
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            read_aloud::read_aloud,
            read_aloud::current_selection,
            read_aloud::stop_reading,
            read_aloud::dismiss_read_aloud
        ])
        .setup(read_aloud::setup)
        .on_window_event(read_aloud::on_window_event)
        .run(tauri::generate_context!())
}
