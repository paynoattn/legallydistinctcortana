//! The tray icon: the voice server's status, Settings, Pause shortcuts, and Quit. Left
//! click opens Settings.

use speech_application::ConnectionStatus;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, Runtime};

use crate::shortcuts::Shortcuts;
use crate::support::log_failure;
use crate::windows;

const SETTINGS_ITEM: &str = "settings";
const PAUSE_ITEM: &str = "pause";
const QUIT_ITEM: &str = "quit";

pub(crate) struct Tray<R: Runtime> {
    icon: TrayIcon<R>,
    status: MenuItem<R>,
}

pub(crate) fn setup<R: Runtime>(app: &App<R>, status: &ConnectionStatus) -> tauri::Result<()> {
    let handle = app.handle();
    let status_item = MenuItem::with_id(handle, "status", &status.summary, false, None::<&str>)?;
    let pause = CheckMenuItem::with_id(
        handle,
        PAUSE_ITEM,
        "Pause shortcuts",
        true,
        false,
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        handle,
        &[
            &status_item,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, SETTINGS_ITEM, "Settings", true, None::<&str>)?,
            &pause,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, QUIT_ITEM, "Quit", true, None::<&str>)?,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip(tooltip(handle, status))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|handle, event| on_menu(handle, &event))
        .on_tray_icon_event(|tray, event| on_click(tray, &event));
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let icon = builder.build(app)?;
    app.manage(Tray {
        icon,
        status: status_item,
    });
    Ok(())
}

/// Shows `status` in the tooltip and the menu.
pub(crate) fn show_status<R: Runtime>(app: &AppHandle<R>, status: &ConnectionStatus) {
    if let Some(tray) = app.try_state::<Tray<R>>() {
        tray.show(app, status);
    }
}

impl<R: Runtime> Tray<R> {
    fn show(&self, app: &AppHandle<R>, status: &ConnectionStatus) {
        let shown = self
            .icon
            .set_tooltip(Some(tooltip(app, status)))
            .and_then(|()| self.status.set_text(&status.summary));
        log_failure(shown, "update the tray");
    }
}

fn tooltip<R: Runtime>(app: &AppHandle<R>, status: &ConnectionStatus) -> String {
    format!("{}: {}", app.package_info().name, status.summary)
}

type Action<R> = fn(&AppHandle<R>);

fn on_menu<R: Runtime>(app: &AppHandle<R>, event: &MenuEvent) {
    let actions: [(&str, Action<R>); 3] = [
        (SETTINGS_ITEM, windows::show_settings),
        (PAUSE_ITEM, toggle_pause),
        (QUIT_ITEM, quit),
    ];
    let chosen = event.id().as_ref();
    if let Some((_, action)) = actions.iter().find(|(item, _)| *item == chosen) {
        action(app);
    }
}

fn toggle_pause<R: Runtime>(app: &AppHandle<R>) {
    let shortcuts = app.state::<Shortcuts>();
    shortcuts.set_paused(app, !shortcuts.is_paused());
}

fn quit<R: Runtime>(app: &AppHandle<R>) {
    app.exit(0);
}

fn on_click<R: Runtime>(tray: &TrayIcon<R>, event: &TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = *event
    {
        windows::show_settings(tray.app_handle());
    }
}
