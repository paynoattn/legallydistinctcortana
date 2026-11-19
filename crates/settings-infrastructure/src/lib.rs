//! Settings adapters.
//!
//! - [`JsonFileStore`]: the settings as a small JSON file (for example
//!   `%APPDATA%\<identifier>\settings.json`), written atomically.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use settings_application::{SettingsInput, SettingsStore, StoreError};

/// Settings kept as JSON in one file. Unknown fields are ignored and missing ones are left
/// to the defaults, so older and newer versions of the app can share the file.
#[derive(Debug, Clone)]
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    /// A store for the file at `path`. Its folder is created on the first save.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Where the settings are kept.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, json: &str) -> std::io::Result<()> {
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        let partial = self.path.with_extension("json.partial");
        fs::write(&partial, json)?;
        fs::rename(&partial, &self.path)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Stored {
    #[serde(skip_serializing_if = "Option::is_none")]
    dictate_shortcut: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_aloud_shortcut: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stt_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tts_port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stt_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reduce_motion: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_overlay: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_at_sign_in: Option<bool>,
}

impl From<Stored> for SettingsInput {
    fn from(stored: Stored) -> Self {
        Self {
            dictate_shortcut: stored.dictate_shortcut,
            read_aloud_shortcut: stored.read_aloud_shortcut,
            voice_host: stored.voice_host,
            stt_port: stored.stt_port,
            tts_port: stored.tts_port,
            stt_language: stored.stt_language,
            reduce_motion: stored.reduce_motion,
            show_overlay: stored.show_overlay,
            start_at_sign_in: stored.start_at_sign_in,
        }
    }
}

impl From<&SettingsInput> for Stored {
    fn from(input: &SettingsInput) -> Self {
        let input = input.clone();
        Self {
            dictate_shortcut: input.dictate_shortcut,
            read_aloud_shortcut: input.read_aloud_shortcut,
            voice_host: input.voice_host,
            stt_port: input.stt_port,
            tts_port: input.tts_port,
            stt_language: input.stt_language,
            reduce_motion: input.reduce_motion,
            show_overlay: input.show_overlay,
            start_at_sign_in: input.start_at_sign_in,
        }
    }
}

impl SettingsStore for JsonFileStore {
    fn load(&self) -> Result<Option<SettingsInput>, StoreError> {
        let json = match fs::read_to_string(&self.path) {
            Ok(json) => json,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError(format!("can't read it: {error}"))),
        };
        serde_json::from_str::<Stored>(&json)
            .map(|stored| Some(stored.into()))
            .map_err(|error| StoreError(format!("it isn't valid settings JSON: {error}")))
    }

    fn save(&self, settings: &SettingsInput) -> Result<(), StoreError> {
        let json = serde_json::to_string_pretty(&Stored::from(settings))
            .map_err(|error| StoreError(format!("can't encode it: {error}")))?;
        self.write(&json)
            .map_err(|error| StoreError(format!("can't write it: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let folder = env::temp_dir().join(format!(
                "settings-store-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            drop(fs::remove_dir_all(&folder));
            Self(folder)
        }

        fn store(&self) -> JsonFileStore {
            JsonFileStore::new(self.0.join("nested").join("settings.json"))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.0));
        }
    }

    fn full() -> SettingsInput {
        SettingsInput {
            dictate_shortcut: Some("F9".into()),
            read_aloud_shortcut: Some("Ctrl+Alt+R".into()),
            voice_host: Some("cosmiccrisp".into()),
            stt_port: Some(10_300),
            tts_port: Some(10_200),
            stt_language: Some("en".into()),
            reduce_motion: Some(true),
            show_overlay: Some(false),
            start_at_sign_in: Some(true),
        }
    }

    #[test]
    fn a_missing_file_holds_nothing() {
        let scratch = Scratch::new();
        assert_eq!(scratch.store().load(), Ok(None));
    }

    #[test]
    fn saved_settings_load_back_and_create_the_folder() {
        let scratch = Scratch::new();
        let store = scratch.store();
        store.save(&full()).unwrap();
        assert_eq!(store.load(), Ok(Some(full())));
        assert!(
            !store.path().with_extension("json.partial").exists(),
            "the partial file is renamed into place"
        );
    }

    #[test]
    fn the_file_is_readable_camel_case_json() {
        let scratch = Scratch::new();
        let store = scratch.store();
        store.save(&full()).unwrap();
        let json = fs::read_to_string(store.path()).unwrap();
        assert!(json.contains("\"voiceHost\": \"cosmiccrisp\""), "{json}");
        assert!(json.contains("\"startAtSignIn\": true"), "{json}");
    }

    #[test]
    fn saving_again_replaces_the_file() {
        let scratch = Scratch::new();
        let store = scratch.store();
        store.save(&full()).unwrap();
        let changed = SettingsInput {
            voice_host: Some("other".into()),
            ..full()
        };
        store.save(&changed).unwrap();
        assert_eq!(store.load(), Ok(Some(changed)));
    }

    #[test]
    fn missing_and_unknown_fields_are_tolerated() {
        let scratch = Scratch::new();
        let store = scratch.store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), r#"{"voiceHost":"box","futureThing":3}"#).unwrap();
        assert_eq!(
            store.load(),
            Ok(Some(SettingsInput {
                voice_host: Some("box".into()),
                ..SettingsInput::default()
            }))
        );
    }

    #[test]
    fn unset_fields_are_left_out_of_the_file() {
        let scratch = Scratch::new();
        let store = scratch.store();
        store.save(&SettingsInput::default()).unwrap();
        assert_eq!(fs::read_to_string(store.path()).unwrap(), "{}");
    }

    #[test]
    fn a_damaged_file_is_reported() {
        let scratch = Scratch::new();
        let store = scratch.store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "{not json").unwrap();
        let error = store.load().unwrap_err();
        assert!(
            error.0.starts_with("it isn't valid settings JSON: "),
            "{error}"
        );
    }

    #[test]
    fn a_path_that_is_a_folder_cannot_be_read_or_written() {
        let scratch = Scratch::new();
        let store = scratch.store();
        fs::create_dir_all(store.path()).unwrap();
        assert!(
            store.load().unwrap_err().0.starts_with("can't read it: "),
            "reading a folder fails"
        );
        assert!(
            store
                .save(&full())
                .unwrap_err()
                .0
                .starts_with("can't write it: "),
            "replacing a folder fails"
        );
    }
}
