//! Tauri shell for the assistant: builds the window from `tauri.conf.json` and runs the
//! event loop. Feature code does not live here; it lives in the feature crates and is
//! wired in by this composition root.

/// Builds and runs the Tauri application until the last window closes.
///
/// # Errors
///
/// Returns the Tauri error if the runtime fails to start or exits abnormally.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default().run(tauri::generate_context!())
}
