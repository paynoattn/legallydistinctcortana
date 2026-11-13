//! Desktop binary. All wiring lives in [`desktop_lib::run`].
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), tauri::Error> {
    desktop_lib::run()
}
