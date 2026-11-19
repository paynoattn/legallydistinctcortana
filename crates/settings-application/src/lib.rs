//! Settings use cases: open the stored settings ([`SettingsService::open`]), put them in
//! effect ([`SettingsService::put_in_effect`]), and save a change
//! ([`SettingsService::save`]), through the [`SettingsStore`] and [`SettingsEffects`] ports.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

pub use settings_domain::{Chord, Field, FieldError, Settings, SettingsInput};
use thiserror::Error;

/// Why the settings file could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct StoreError(pub String);

/// Keeps the settings between runs.
pub trait SettingsStore: Send + Sync {
    /// What was stored, or `None` when nothing has been stored yet.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the stored settings cannot be read.
    fn load(&self) -> Result<Option<SettingsInput>, StoreError>;

    /// Replaces what is stored with `settings`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when they cannot be written.
    fn save(&self, settings: &SettingsInput) -> Result<(), StoreError>;
}

/// Puts settings in effect in the running app.
pub trait SettingsEffects {
    /// Shows or hides the corner indicator, and sets how it moves.
    fn indicator(&self, settings: &Settings);

    /// Points the speech services at the voice server in `settings`.
    fn voice_server(&self, settings: &Settings);

    /// Adds the app to the sign-in items (`enabled`) or removes it.
    ///
    /// # Errors
    ///
    /// Why the system refused, in one sentence for the person changing it.
    fn start_at_sign_in(&self, enabled: bool) -> Result<(), String>;

    /// Registers the shortcuts in `settings`, replacing the ones registered before.
    /// Returns the ones that could not be registered.
    fn shortcuts(&self, settings: &Settings) -> Vec<FieldError>;
}

/// Why a change was not saved.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SaveError {
    /// Some fields were refused; nothing changed.
    #[error("{} settings were refused", .0.len())]
    Invalid(Vec<FieldError>),
    /// The system refused part of the change (another app holds a shortcut, or the sign-in
    /// items could not be changed); nothing changed.
    #[error("{} settings could not be put in effect", .0.len())]
    Refused(Vec<FieldError>),
    /// The change was valid but could not be stored; nothing changed.
    #[error("can't save the settings: {0}")]
    Store(#[from] StoreError),
}

/// A saved change: the settings before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// What was in effect.
    pub before: Settings,
    /// What is in effect now.
    pub after: Settings,
}

impl Change {
    /// Whether either shortcut changed, so they must be registered again.
    #[must_use]
    pub fn shortcuts_changed(&self) -> bool {
        self.before.dictate_shortcut != self.after.dictate_shortcut
            || self.before.read_aloud_shortcut != self.after.read_aloud_shortcut
    }

    /// Whether the voice server's host, ports, or language changed, so the speech services
    /// must be rebuilt.
    #[must_use]
    pub fn voice_server_changed(&self) -> bool {
        self.before.voice_host != self.after.voice_host
            || self.before.stt_port != self.after.stt_port
            || self.before.tts_port != self.after.tts_port
            || self.before.stt_language != self.after.stt_language
    }

    /// Whether starting at sign-in was turned on or off.
    #[must_use]
    pub const fn autostart_changed(&self) -> bool {
        self.before.start_at_sign_in != self.after.start_at_sign_in
    }

    /// Puts in effect the parts of the change the system may refuse: the sign-in entry and
    /// the shortcuts. Returns what was refused.
    fn claim(&self, effects: &dyn SettingsEffects) -> Vec<FieldError> {
        let mut refused = Vec::new();
        if self.autostart_changed()
            && let Err(message) = effects.start_at_sign_in(self.after.start_at_sign_in)
        {
            refused.push(FieldError {
                field: Field::START_AT_SIGN_IN,
                message,
            });
        }
        if self.shortcuts_changed() {
            refused.extend(effects.shortcuts(&self.after));
        }
        refused
    }

    /// Takes back what [`Change::claim`] put in effect.
    fn release(&self, effects: &dyn SettingsEffects) {
        let back = Self {
            before: self.after.clone(),
            after: self.before.clone(),
        };
        drop(back.claim(effects));
    }

    /// Puts in effect the parts of the change that cannot be refused, once it is stored.
    fn settle(&self, effects: &dyn SettingsEffects) {
        effects.indicator(&self.after);
        if self.voice_server_changed() {
            effects.voice_server(&self.after);
        }
    }
}

/// Something wrong with the stored settings that was worked around when opening them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenWarning {
    /// The store could not be read, so the defaults are in effect.
    Unreadable(StoreError),
    /// A stored field was refused, so its default is in effect.
    Refused(FieldError),
}

impl fmt::Display for OpenWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(error) => {
                write!(
                    f,
                    "the stored settings can't be read ({error}); using the defaults"
                )
            }
            Self::Refused(error) => write!(
                f,
                "the stored {} was refused ({}); using its default",
                error.field.id(),
                error.message
            ),
        }
    }
}

/// The settings in effect, kept in step with the [`SettingsStore`]. Safe to share between
/// threads.
pub struct SettingsService {
    store: Arc<dyn SettingsStore>,
    defaults: Settings,
    current: Mutex<Settings>,
    saving: Mutex<()>,
}

impl fmt::Debug for SettingsService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SettingsService")
            .field("current", &*lock(&self.current))
            .finish_non_exhaustive()
    }
}

impl SettingsService {
    /// Opens the settings in `store`, filling anything missing or refused from `defaults`.
    /// Never fails: problems with the stored settings come back as warnings, and the
    /// defaults stand in for whatever could not be used.
    #[must_use]
    pub fn open(store: Arc<dyn SettingsStore>, defaults: Settings) -> (Self, Vec<OpenWarning>) {
        let (current, warnings) = match store.load() {
            Ok(None) => (defaults.clone(), Vec::new()),
            Ok(Some(stored)) => {
                let (settings, refused) = defaults.repaired(&stored);
                (
                    settings,
                    refused.into_iter().map(OpenWarning::Refused).collect(),
                )
            }
            Err(error) => (defaults.clone(), vec![OpenWarning::Unreadable(error)]),
        };
        let service = Self {
            store,
            defaults,
            current: Mutex::new(current),
            saving: Mutex::new(()),
        };
        (service, warnings)
    }

    /// The settings in effect.
    #[must_use]
    pub fn current(&self) -> Settings {
        lock(&self.current).clone()
    }

    /// The settings a new install starts with.
    #[must_use]
    pub fn defaults(&self) -> Settings {
        self.defaults.clone()
    }

    /// Puts the settings in effect at startup: the indicator, the sign-in entry, and the
    /// shortcuts. The speech services are built from [`SettingsService::current`]. Returns
    /// the shortcuts that could not be registered.
    pub fn put_in_effect(&self, effects: &dyn SettingsEffects) -> Vec<FieldError> {
        let settings = self.current();
        effects.indicator(&settings);
        let sign_in = effects
            .start_at_sign_in(settings.start_at_sign_in)
            .err()
            .map(|message| FieldError {
                field: Field::START_AT_SIGN_IN,
                message,
            });
        sign_in
            .into_iter()
            .chain(effects.shortcuts(&settings))
            .collect()
    }

    /// Validates `input` against the settings in effect, registers the parts the system may
    /// refuse (shortcuts, the sign-in entry), stores the result, and only then puts the rest
    /// in effect (the indicator, the voice server). Returns the settings now in effect.
    ///
    /// # Errors
    ///
    /// [`SaveError::Invalid`] with every refused field, [`SaveError::Refused`] with what the
    /// system refused, or [`SaveError::Store`]. In every case the settings in effect and in
    /// the store stay as they were, and the voice server is not touched.
    pub fn save(
        &self,
        input: &SettingsInput,
        effects: &dyn SettingsEffects,
    ) -> Result<Settings, SaveError> {
        let _saving = lock(&self.saving);
        let before = self.current();
        let after = before.with(input).map_err(SaveError::Invalid)?;
        let change = Change { before, after };
        let refused = change.claim(effects);
        if !refused.is_empty() {
            change.release(effects);
            return Err(SaveError::Refused(refused));
        }
        if let Err(error) = self.store.save(&change.after.to_input()) {
            change.release(effects);
            return Err(error.into());
        }
        lock(&self.current).clone_from(&change.after);
        change.settle(effects);
        Ok(change.after)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Memory {
        stored: Mutex<Option<SettingsInput>>,
        unreadable: bool,
        read_only: bool,
    }

    impl Memory {
        fn holding(input: SettingsInput) -> Self {
            Self {
                stored: Mutex::new(Some(input)),
                ..Self::default()
            }
        }

        fn stored(&self) -> Option<SettingsInput> {
            self.stored.lock().unwrap().clone()
        }
    }

    impl SettingsStore for Memory {
        fn load(&self) -> Result<Option<SettingsInput>, StoreError> {
            if self.unreadable {
                return Err(StoreError("not JSON".into()));
            }
            Ok(self.stored())
        }

        fn save(&self, settings: &SettingsInput) -> Result<(), StoreError> {
            if self.read_only {
                return Err(StoreError("access denied".into()));
            }
            *self.stored.lock().unwrap() = Some(settings.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct Running {
        calls: Mutex<Vec<String>>,
        taken: Option<&'static str>,
        no_sign_in: bool,
    }

    impl Running {
        fn holding(chord: &'static str) -> Self {
            Self {
                taken: Some(chord),
                ..Self::default()
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        fn record(&self, call: String) {
            self.calls.lock().unwrap().push(call);
        }
    }

    impl SettingsEffects for Running {
        fn indicator(&self, settings: &Settings) {
            self.record(format!(
                "indicator {} {}",
                settings.show_overlay, settings.reduce_motion
            ));
        }

        fn voice_server(&self, settings: &Settings) {
            self.record(format!("server {}", settings.voice_host));
        }

        fn start_at_sign_in(&self, enabled: bool) -> Result<(), String> {
            self.record(format!("sign-in {enabled}"));
            if self.no_sign_in {
                return Err("Windows refused.".into());
            }
            Ok(())
        }

        fn shortcuts(&self, settings: &Settings) -> Vec<FieldError> {
            let dictate = settings.dictate_shortcut.to_string();
            self.record(format!(
                "shortcuts {dictate} {}",
                settings.read_aloud_shortcut
            ));
            self.taken
                .filter(|taken| *taken == dictate)
                .map(|taken| FieldError {
                    field: Field::DICTATE_SHORTCUT,
                    message: format!("{taken} is taken."),
                })
                .into_iter()
                .collect()
        }
    }

    fn defaults() -> Settings {
        Settings::defaults(|_| None)
    }

    fn open(store: &Arc<Memory>) -> (SettingsService, Vec<OpenWarning>) {
        let store: Arc<dyn SettingsStore> = Arc::<Memory>::clone(store);
        SettingsService::open(store, defaults())
    }

    fn host(name: &str) -> SettingsInput {
        SettingsInput {
            voice_host: Some(name.into()),
            ..SettingsInput::default()
        }
    }

    #[test]
    fn nothing_stored_means_the_defaults() {
        let (service, warnings) = open(&Arc::new(Memory::default()));
        assert_eq!(service.current(), defaults());
        assert_eq!(service.defaults(), defaults());
        assert!(warnings.is_empty(), "nothing to warn about");
    }

    #[test]
    fn stored_settings_are_in_effect() {
        let (service, warnings) = open(&Arc::new(Memory::holding(host("box"))));
        assert_eq!(service.current().voice_host, "box");
        assert!(warnings.is_empty(), "all valid");
    }

    #[test]
    fn a_refused_stored_field_falls_back_with_a_warning() {
        let stored = SettingsInput {
            stt_port: Some(0),
            ..host("box")
        };
        let (service, warnings) = open(&Arc::new(Memory::holding(stored)));
        assert_eq!(service.current().voice_host, "box");
        assert_eq!(service.current().stt_port, 10_300);
        assert_eq!(
            warnings.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["the stored sttPort was refused (Use a port from 1 to 65535.); using its default"]
        );
    }

    #[test]
    fn an_unreadable_store_means_the_defaults_with_a_warning() {
        let store = Arc::new(Memory {
            unreadable: true,
            ..Memory::default()
        });
        let (service, warnings) = open(&store);
        assert_eq!(service.current(), defaults());
        assert_eq!(
            warnings.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["the stored settings can't be read (not JSON); using the defaults"]
        );
    }

    #[test]
    fn a_valid_change_is_stored_and_in_effect() {
        let store = Arc::new(Memory::default());
        let (service, _) = open(&store);
        let running = Running::default();
        let after = service.save(&host("cosmiccrisp"), &running).unwrap();
        assert_eq!(after.voice_host, "cosmiccrisp");
        assert_eq!(service.current(), after);
        assert_eq!(store.stored(), Some(after.to_input()));
        assert_eq!(
            running.calls(),
            ["indicator true false", "server cosmiccrisp"]
        );
    }

    #[test]
    fn only_what_changed_is_applied_again() {
        let (service, _) = open(&Arc::new(Memory::default()));
        let running = Running::default();
        let input = SettingsInput {
            start_at_sign_in: Some(true),
            read_aloud_shortcut: Some("Ctrl+Shift+R".into()),
            reduce_motion: Some(true),
            ..SettingsInput::default()
        };
        service.save(&input, &running).unwrap();
        assert_eq!(
            running.calls(),
            [
                "sign-in true",
                "shortcuts F9 Ctrl+Shift+R",
                "indicator true true"
            ]
        );
    }

    #[test]
    fn a_shortcut_another_app_holds_puts_the_shortcuts_back_and_keeps_the_server() {
        let store = Arc::new(Memory::default());
        let (service, _) = open(&store);
        let running = Running::holding("F8");
        let input = SettingsInput {
            dictate_shortcut: Some("F8".into()),
            ..host("box")
        };
        assert_eq!(
            service.save(&input, &running),
            Err(SaveError::Refused(vec![FieldError {
                field: Field::DICTATE_SHORTCUT,
                message: "F8 is taken.".into(),
            }]))
        );
        assert_eq!(service.current(), defaults());
        assert_eq!(store.stored(), None);
        assert_eq!(
            running.calls(),
            ["shortcuts F8 Ctrl+Alt+R", "shortcuts F9 Ctrl+Alt+R"]
        );
    }

    #[test]
    fn a_refused_sign_in_entry_is_reported_on_its_field() {
        let store = Arc::new(Memory::default());
        let (service, _) = open(&store);
        let running = Running {
            no_sign_in: true,
            ..Running::default()
        };
        let input = SettingsInput {
            start_at_sign_in: Some(true),
            ..SettingsInput::default()
        };
        assert_eq!(
            service.save(&input, &running),
            Err(SaveError::Refused(vec![FieldError {
                field: Field::START_AT_SIGN_IN,
                message: "Windows refused.".into(),
            }]))
        );
        assert_eq!(service.current(), defaults());
        assert_eq!(store.stored(), None);
        assert_eq!(running.calls(), ["sign-in true", "sign-in false"]);
        assert_eq!(
            service.put_in_effect(&running),
            [FieldError {
                field: Field::START_AT_SIGN_IN,
                message: "Windows refused.".into(),
            }]
        );
    }

    #[test]
    fn startup_puts_the_indicator_sign_in_and_shortcuts_in_effect() {
        let (service, _) = open(&Arc::new(Memory::default()));
        let running = Running::holding("F9");
        assert_eq!(
            service.put_in_effect(&running),
            [FieldError {
                field: Field::DICTATE_SHORTCUT,
                message: "F9 is taken.".into(),
            }]
        );
        assert_eq!(
            running.calls(),
            [
                "indicator true false",
                "sign-in false",
                "shortcuts F9 Ctrl+Alt+R"
            ]
        );
    }

    #[test]
    fn a_refused_change_changes_nothing() {
        let store = Arc::new(Memory::default());
        let (service, _) = open(&store);
        let running = Running::default();
        let refused = service.save(&host("my box"), &running);
        assert_eq!(
            refused,
            Err(SaveError::Invalid(vec![FieldError {
                field: Field::VOICE_HOST,
                message: "A host name can't contain spaces.".into(),
            }]))
        );
        assert_eq!(service.current(), defaults());
        assert_eq!(store.stored(), None);
        assert!(running.calls().is_empty(), "nothing applied");
    }

    #[test]
    fn a_change_that_cannot_be_stored_is_not_in_effect() {
        let store = Arc::new(Memory {
            read_only: true,
            ..Memory::default()
        });
        let (service, _) = open(&store);
        let running = Running::default();
        let input = SettingsInput {
            dictate_shortcut: Some("F8".into()),
            ..host("box")
        };
        let failed = service.save(&input, &running);
        assert_eq!(
            failed,
            Err(SaveError::Store(StoreError("access denied".into())))
        );
        assert_eq!(service.current(), defaults());
        assert_eq!(
            running.calls(),
            ["shortcuts F8 Ctrl+Alt+R", "shortcuts F9 Ctrl+Alt+R"]
        );
    }

    #[test]
    fn save_errors_are_readable() {
        assert_eq!(
            SaveError::Store(StoreError("disk full".into())).to_string(),
            "can't save the settings: disk full"
        );
        assert_eq!(
            SaveError::Invalid(Vec::new()).to_string(),
            "0 settings were refused"
        );
        assert_eq!(
            SaveError::Refused(Vec::new()).to_string(),
            "0 settings could not be put in effect"
        );
    }

    #[test]
    fn a_change_says_what_must_be_applied_again() {
        let before = defaults();
        let change = |input: SettingsInput| Change {
            before: before.clone(),
            after: before.with(&input).unwrap(),
        };
        let shortcut = change(SettingsInput {
            read_aloud_shortcut: Some("Ctrl+Shift+R".into()),
            ..SettingsInput::default()
        });
        let dictate = change(SettingsInput {
            dictate_shortcut: Some("F8".into()),
            ..SettingsInput::default()
        });
        let server = change(host("box"));
        let autostart = change(SettingsInput {
            start_at_sign_in: Some(true),
            ..SettingsInput::default()
        });
        assert_eq!(
            [&shortcut, &dictate, &server, &autostart].map(|saved| (
                saved.shortcuts_changed(),
                saved.voice_server_changed(),
                saved.autostart_changed()
            )),
            [
                (true, false, false),
                (true, false, false),
                (false, true, false),
                (false, false, true),
            ]
        );
    }

    #[test]
    fn each_server_field_counts_as_a_server_change() {
        let before = defaults();
        let inputs = [
            SettingsInput {
                stt_port: Some(1),
                ..SettingsInput::default()
            },
            SettingsInput {
                tts_port: Some(1),
                ..SettingsInput::default()
            },
            SettingsInput {
                stt_language: Some("de".into()),
                ..SettingsInput::default()
            },
            SettingsInput {
                reduce_motion: Some(true),
                ..SettingsInput::default()
            },
        ];
        assert_eq!(
            inputs.map(|input| Change {
                before: before.clone(),
                after: before.with(&input).unwrap(),
            }
            .voice_server_changed()),
            [true, true, true, false]
        );
    }

    #[test]
    fn debug_shows_the_settings_in_effect() {
        let (service, _) = open(&Arc::new(Memory::holding(host("debugbox"))));
        assert!(
            format!("{service:?}").contains("debugbox"),
            "debug output shows the current settings"
        );
    }
}
