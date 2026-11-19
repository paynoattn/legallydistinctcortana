//! The global shortcuts, registered from the settings. They can be paused from the tray and
//! suspended while the settings window records a new chord.

use std::sync::atomic::{AtomicBool, Ordering};

use settings_application::{Chord, Field, FieldError, Settings};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::dictation::DictationKeys;
use crate::read_aloud;
use crate::support::log_failure;

#[derive(Default)]
pub(crate) struct Shortcuts {
    paused: AtomicBool,
    suspended: AtomicBool,
}

impl Shortcuts {
    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub(crate) fn set_paused<R: Runtime>(&self, app: &AppHandle<R>, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
        tracing::info!(paused, "shortcuts paused from the tray");
        log_refused(self.refresh(app, &current(app)));
    }

    pub(crate) fn set_suspended<R: Runtime>(&self, app: &AppHandle<R>, suspended: bool) {
        self.suspended.store(suspended, Ordering::SeqCst);
        log_refused(self.refresh(app, &current(app)));
    }

    /// Registers the shortcuts in `settings` unless paused or suspended, replacing any
    /// registered before. A dictation key held down is let go first, since its release
    /// would go to no one. Returns the shortcuts that could not be registered.
    pub(crate) fn refresh<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        settings: &Settings,
    ) -> Vec<FieldError> {
        app.state::<DictationKeys>().key(false);
        log_failure(
            app.global_shortcut().unregister_all(),
            "release the shortcuts",
        );
        if self.is_held() {
            return Vec::new();
        }
        [
            register_dictation(app, &settings.dictate_shortcut),
            register_read_aloud(app, &settings.read_aloud_shortcut),
        ]
        .into_iter()
        .filter_map(Result::err)
        .collect()
    }

    fn is_held(&self) -> bool {
        self.paused.load(Ordering::SeqCst) || self.suspended.load(Ordering::SeqCst)
    }
}

/// Logs the shortcuts that could not be registered when nothing can show them inline.
pub(crate) fn log_refused(refused: Vec<FieldError>) {
    for error in refused {
        tracing::warn!(%error, "a shortcut is off");
    }
}

fn current<R: Runtime>(app: &AppHandle<R>) -> Settings {
    app.state::<crate::settings::SettingsState>().current()
}

fn register_dictation<R: Runtime>(app: &AppHandle<R>, chord: &Chord) -> Result<(), FieldError> {
    register(
        app,
        Binding(chord, Field::DICTATE_SHORTCUT),
        |app, pressed| {
            app.state::<DictationKeys>().key(pressed);
        },
    )
}

fn register_read_aloud<R: Runtime>(app: &AppHandle<R>, chord: &Chord) -> Result<(), FieldError> {
    register(
        app,
        Binding(chord, Field::READ_ALOUD_SHORTCUT),
        |app, pressed| {
            if !pressed {
                read_aloud::on_shortcut(app);
            }
        },
    )
}

struct Binding<'a>(&'a Chord, Field);

fn register<R: Runtime>(
    app: &AppHandle<R>,
    Binding(chord, field): Binding<'_>,
    on_key: impl Fn(&AppHandle<R>, bool) + Send + Sync + 'static,
) -> Result<(), FieldError> {
    let refused = |message: String| FieldError { field, message };
    let shortcut: Shortcut = chord
        .accelerator()
        .parse()
        .map_err(|_| refused(format!("{chord} can't be used as a global shortcut.")))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            on_key(app, event.state() == ShortcutState::Pressed);
        })
        .map_err(|error| {
            tracing::warn!(%error, %chord, "could not register a shortcut");
            refused(format!("{chord} is taken by another app."))
        })?;
    tracing::info!(%chord, field = field.id(), "shortcut registered");
    Ok(())
}
