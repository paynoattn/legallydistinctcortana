//! The main and settings windows: shown on demand, hidden instead of closed so the app
//! stays in the tray.

use tauri::{
    AppHandle, CloseRequestApi, Manager, Runtime, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent,
};

use crate::support::log_failure;

pub(crate) const MAIN: &str = "main";
pub(crate) const SETTINGS: &str = "settings";

/// Launch flag that starts the app in the tray with no window (used at sign-in).
pub(crate) const BACKGROUND_FLAG: &str = "--background";

/// Creates the windows marked `"create": false` in `tauri.conf.json`. Their pages call
/// commands as soon as they load, so they are created once the state those commands use is
/// managed, rather than with the others before setup runs.
pub(crate) fn create_deferred<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    for config in app
        .config()
        .app
        .windows
        .iter()
        .filter(|config| !config.create)
    {
        WebviewWindowBuilder::from_config(app, config)?.build()?;
    }
    Ok(())
}

pub(crate) fn show_settings<R: Runtime>(app: &AppHandle<R>) {
    show(app, SETTINGS);
}

pub(crate) fn show_main<R: Runtime>(app: &AppHandle<R>) {
    show(app, MAIN);
}

fn show<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        bring_forward(&window);
    } else {
        tracing::warn!(label, "this window is missing from tauri.conf.json");
    }
}

fn bring_forward<R: Runtime>(window: &WebviewWindow<R>) {
    let shown = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    log_failure(shown, "show the window");
}

/// Closing the main or settings window hides it; Quit in the tray exits.
pub(crate) fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        hide_instead(window, api);
    }
}

fn hide_instead<R: Runtime>(window: &Window<R>, api: &CloseRequestApi) {
    if [MAIN, SETTINGS].contains(&window.label()) {
        api.prevent_close();
        log_failure(window.hide(), "hide the window");
    }
}
