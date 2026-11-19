use std::fmt;

use crate::chord::Chord;

/// Dictation shortcut when `DICTATE_SHORTCUT` is not set.
pub const DEFAULT_DICTATE_SHORTCUT: &str = "F9";

/// Read-aloud shortcut when `READ_ALOUD_SHORTCUT` is not set.
pub const DEFAULT_READ_ALOUD_SHORTCUT: &str = "Ctrl+Alt+R";

/// Voice host when `VOICE_HOST` is not set (the same default as `speech-infrastructure`).
pub const DEFAULT_VOICE_HOST: &str = "localhost";

/// Speech recognition (Wyoming Whisper) port when `VOICE_STT_PORT` is not set.
pub const DEFAULT_STT_PORT: u16 = 10_300;

/// Voice (Wyoming Piper) port when `VOICE_TTS_PORT` is not set.
pub const DEFAULT_TTS_PORT: u16 = 10_200;

/// Dictation language when `VOICE_STT_LANGUAGE` is not set.
pub const DEFAULT_STT_LANGUAGE: &str = "en";

/// Longest host name accepted (the DNS limit).
pub const MAX_HOST_CHARS: usize = 253;

/// One setting, named as the settings window names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Field(&'static str);

impl Field {
    /// Hold to dictate.
    pub const DICTATE_SHORTCUT: Self = Self("dictateShortcut");
    /// Read the selection aloud.
    pub const READ_ALOUD_SHORTCUT: Self = Self("readAloudShortcut");
    /// The computer running the voice servers.
    pub const VOICE_HOST: Self = Self("voiceHost");
    /// The speech recognition server's port.
    pub const STT_PORT: Self = Self("sttPort");
    /// The voice server's port.
    pub const TTS_PORT: Self = Self("ttsPort");
    /// The language dictation listens for.
    pub const STT_LANGUAGE: Self = Self("sttLanguage");
    /// Fades instead of moving.
    pub const REDUCE_MOTION: Self = Self("reduceMotion");
    /// Show the corner indicator while dictating or reading.
    pub const SHOW_OVERLAY: Self = Self("showOverlay");
    /// Start hidden in the tray when signing in to Windows.
    pub const START_AT_SIGN_IN: Self = Self("startAtSignIn");

    /// The field's name in the settings file and the settings window.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.0
    }
}

/// Why one field's value was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The field.
    pub field: Field,
    /// What is wrong, for the person changing it.
    pub message: String,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field.id(), self.message)
    }
}

/// Raw values from the settings window or the settings file. `None` keeps the current
/// value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsInput {
    /// See [`Field::DICTATE_SHORTCUT`].
    pub dictate_shortcut: Option<String>,
    /// See [`Field::READ_ALOUD_SHORTCUT`].
    pub read_aloud_shortcut: Option<String>,
    /// See [`Field::VOICE_HOST`].
    pub voice_host: Option<String>,
    /// See [`Field::STT_PORT`].
    pub stt_port: Option<i64>,
    /// See [`Field::TTS_PORT`].
    pub tts_port: Option<i64>,
    /// See [`Field::STT_LANGUAGE`].
    pub stt_language: Option<String>,
    /// See [`Field::REDUCE_MOTION`].
    pub reduce_motion: Option<bool>,
    /// See [`Field::SHOW_OVERLAY`].
    pub show_overlay: Option<bool>,
    /// See [`Field::START_AT_SIGN_IN`].
    pub start_at_sign_in: Option<bool>,
}

/// Every setting, each one valid, and the two shortcuts different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// See [`Field::DICTATE_SHORTCUT`].
    pub dictate_shortcut: Chord,
    /// See [`Field::READ_ALOUD_SHORTCUT`].
    pub read_aloud_shortcut: Chord,
    /// See [`Field::VOICE_HOST`].
    pub voice_host: String,
    /// See [`Field::STT_PORT`].
    pub stt_port: u16,
    /// See [`Field::TTS_PORT`].
    pub tts_port: u16,
    /// See [`Field::STT_LANGUAGE`]: `auto` or a language code such as `en`.
    pub stt_language: String,
    /// See [`Field::REDUCE_MOTION`].
    pub reduce_motion: bool,
    /// See [`Field::SHOW_OVERLAY`].
    pub show_overlay: bool,
    /// See [`Field::START_AT_SIGN_IN`].
    pub start_at_sign_in: bool,
}

impl Settings {
    /// The defaults, taking the shortcuts and the voice server from `lookup` (normally the
    /// process environment: `DICTATE_SHORTCUT`, `READ_ALOUD_SHORTCUT`, `VOICE_HOST`,
    /// `VOICE_STT_PORT`, `VOICE_TTS_PORT`, `VOICE_STT_LANGUAGE`) where those are set and
    /// valid.
    #[must_use]
    pub fn defaults(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let port = |name: &str| lookup(name).and_then(|value| value.trim().parse().ok());
        let environment = SettingsInput {
            dictate_shortcut: lookup("DICTATE_SHORTCUT"),
            read_aloud_shortcut: lookup("READ_ALOUD_SHORTCUT"),
            voice_host: lookup("VOICE_HOST"),
            stt_port: port("VOICE_STT_PORT"),
            tts_port: port("VOICE_TTS_PORT"),
            stt_language: lookup("VOICE_STT_LANGUAGE"),
            ..SettingsInput::default()
        };
        Self::built_in().repaired(&environment).0
    }

    /// The voice server value these settings give the environment variable `name` that
    /// [`Settings::defaults`] reads (`VOICE_HOST`, `VOICE_STT_PORT`, `VOICE_TTS_PORT`,
    /// `VOICE_STT_LANGUAGE`), for adapters configured that way. `None` for any other name.
    #[must_use]
    pub fn variable(&self, name: &str) -> Option<String> {
        match name {
            "VOICE_HOST" => Some(self.voice_host.clone()),
            "VOICE_STT_PORT" => Some(self.stt_port.to_string()),
            "VOICE_TTS_PORT" => Some(self.tts_port.to_string()),
            "VOICE_STT_LANGUAGE" => Some(self.stt_language.clone()),
            _ => None,
        }
    }

    fn built_in() -> Self {
        Self {
            dictate_shortcut: Chord::default_dictate(),
            read_aloud_shortcut: Chord::default_read_aloud(),
            voice_host: DEFAULT_VOICE_HOST.into(),
            stt_port: DEFAULT_STT_PORT,
            tts_port: DEFAULT_TTS_PORT,
            stt_language: DEFAULT_STT_LANGUAGE.into(),
            reduce_motion: false,
            show_overlay: true,
            start_at_sign_in: false,
        }
    }

    /// These settings changed by `input`.
    ///
    /// # Errors
    ///
    /// One [`FieldError`] per refused field; nothing changes unless every field is valid.
    pub fn with(&self, input: &SettingsInput) -> Result<Self, Vec<FieldError>> {
        let (settings, errors) = self.repaired(input);
        if errors.is_empty() {
            Ok(settings)
        } else {
            Err(errors)
        }
    }

    /// These settings changed by every valid field of `input`, keeping the current value of
    /// each refused field, plus what was refused. For reading a settings file that an older
    /// or newer version wrote.
    #[must_use]
    pub fn repaired(&self, input: &SettingsInput) -> (Self, Vec<FieldError>) {
        let mut checked = Checked::default();
        let settings = Self {
            dictate_shortcut: checked
                .pick(
                    Field::DICTATE_SHORTCUT,
                    input.dictate_shortcut.as_deref().map(parse_chord),
                )
                .unwrap_or_else(|| self.dictate_shortcut.clone()),
            read_aloud_shortcut: checked
                .pick(
                    Field::READ_ALOUD_SHORTCUT,
                    input.read_aloud_shortcut.as_deref().map(parse_chord),
                )
                .unwrap_or_else(|| self.read_aloud_shortcut.clone()),
            voice_host: checked
                .pick(Field::VOICE_HOST, input.voice_host.as_deref().map(host))
                .unwrap_or_else(|| self.voice_host.clone()),
            stt_port: checked
                .pick(Field::STT_PORT, input.stt_port.map(port))
                .unwrap_or(self.stt_port),
            tts_port: checked
                .pick(Field::TTS_PORT, input.tts_port.map(port))
                .unwrap_or(self.tts_port),
            stt_language: checked
                .pick(
                    Field::STT_LANGUAGE,
                    input.stt_language.as_deref().map(language),
                )
                .unwrap_or_else(|| self.stt_language.clone()),
            reduce_motion: input.reduce_motion.unwrap_or(self.reduce_motion),
            show_overlay: input.show_overlay.unwrap_or(self.show_overlay),
            start_at_sign_in: input.start_at_sign_in.unwrap_or(self.start_at_sign_in),
        };
        let settings = self.with_distinct_shortcuts(settings, &mut checked);
        (settings, checked.errors)
    }

    fn with_distinct_shortcuts(&self, mut settings: Self, checked: &mut Checked) -> Self {
        if settings.dictate_shortcut == settings.read_aloud_shortcut {
            checked.errors.push(FieldError {
                field: Field::READ_ALOUD_SHORTCUT,
                message: "Already used by Dictate.".into(),
            });
            settings.dictate_shortcut = self.dictate_shortcut.clone();
            settings.read_aloud_shortcut = self.read_aloud_shortcut.clone();
        }
        settings
    }

    /// Every setting as raw values, for storing.
    #[must_use]
    pub fn to_input(&self) -> SettingsInput {
        SettingsInput {
            dictate_shortcut: Some(self.dictate_shortcut.to_string()),
            read_aloud_shortcut: Some(self.read_aloud_shortcut.to_string()),
            voice_host: Some(self.voice_host.clone()),
            stt_port: Some(i64::from(self.stt_port)),
            tts_port: Some(i64::from(self.tts_port)),
            stt_language: Some(self.stt_language.clone()),
            reduce_motion: Some(self.reduce_motion),
            show_overlay: Some(self.show_overlay),
            start_at_sign_in: Some(self.start_at_sign_in),
        }
    }
}

#[derive(Default)]
struct Checked {
    errors: Vec<FieldError>,
}

impl Checked {
    fn pick<T>(&mut self, field: Field, parsed: Option<Result<T, String>>) -> Option<T> {
        match parsed? {
            Ok(value) => Some(value),
            Err(message) => {
                self.errors.push(FieldError { field, message });
                None
            }
        }
    }
}

fn parse_chord(text: &str) -> Result<Chord, String> {
    text.parse()
        .map_err(|error: crate::ChordError| error.to_string())
}

fn host(text: &str) -> Result<String, String> {
    let host = text.trim();
    if host.is_empty() {
        return Err("Enter the name or address of the computer running the voice servers.".into());
    }
    if host.chars().count() > MAX_HOST_CHARS {
        return Err(format!(
            "Use a host name of at most {MAX_HOST_CHARS} characters."
        ));
    }
    if let Some(bad) = host.chars().find(|c| !is_host_char(*c)) {
        return Err(if bad.is_whitespace() {
            "A host name can't contain spaces.".into()
        } else {
            format!("A host name can't contain {bad}.")
        });
    }
    Ok(host.into())
}

fn is_host_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '[' | ']' | '%')
}

fn port(value: i64) -> Result<u16, String> {
    u16::try_from(value)
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "Use a port from 1 to 65535.".into())
}

fn language(text: &str) -> Result<String, String> {
    let code = text.trim().to_ascii_lowercase();
    let is_code = (2..=3).contains(&code.len()) && code.chars().all(|c| c.is_ascii_lowercase());
    if code == "auto" || is_code {
        Ok(code)
    } else {
        Err("Use auto, or a language code such as en.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<String> {
        None
    }

    fn defaults() -> Settings {
        Settings::defaults(none)
    }

    impl SettingsInput {
        fn or(self, fallback: &Self) -> Self {
            Self {
                dictate_shortcut: self
                    .dictate_shortcut
                    .or_else(|| fallback.dictate_shortcut.clone()),
                read_aloud_shortcut: self
                    .read_aloud_shortcut
                    .or_else(|| fallback.read_aloud_shortcut.clone()),
                voice_host: self.voice_host.or_else(|| fallback.voice_host.clone()),
                stt_port: self.stt_port.or(fallback.stt_port),
                tts_port: self.tts_port.or(fallback.tts_port),
                stt_language: self.stt_language.or_else(|| fallback.stt_language.clone()),
                reduce_motion: self.reduce_motion.or(fallback.reduce_motion),
                show_overlay: self.show_overlay.or(fallback.show_overlay),
                start_at_sign_in: self.start_at_sign_in.or(fallback.start_at_sign_in),
            }
        }
    }

    fn error(field: Field, message: &str) -> FieldError {
        FieldError {
            field,
            message: message.into(),
        }
    }

    #[test]
    fn built_in_defaults() {
        assert_eq!(
            defaults().to_input(),
            SettingsInput {
                dictate_shortcut: Some(DEFAULT_DICTATE_SHORTCUT.into()),
                read_aloud_shortcut: Some(DEFAULT_READ_ALOUD_SHORTCUT.into()),
                voice_host: Some("localhost".into()),
                stt_port: Some(10_300),
                tts_port: Some(10_200),
                stt_language: Some("en".into()),
                reduce_motion: Some(false),
                show_overlay: Some(true),
                start_at_sign_in: Some(false),
            }
        );
        assert_eq!(
            [DEFAULT_DICTATE_SHORTCUT, DEFAULT_READ_ALOUD_SHORTCUT],
            ["F9", "Ctrl+Alt+R"]
        );
    }

    #[test]
    fn the_voice_server_reads_back_under_its_variable_names() {
        let settings = defaults()
            .with(&SettingsInput {
                voice_host: Some("cosmiccrisp".into()),
                stt_port: Some(1),
                tts_port: Some(2),
                stt_language: Some("de".into()),
                ..SettingsInput::default()
            })
            .unwrap();
        assert_eq!(
            [
                "VOICE_HOST",
                "VOICE_STT_PORT",
                "VOICE_TTS_PORT",
                "VOICE_STT_LANGUAGE",
                "VOICE_TTS_VOICE",
            ]
            .map(|name| settings.variable(name)),
            [
                Some("cosmiccrisp".into()),
                Some("1".into()),
                Some("2".into()),
                Some("de".into()),
                None,
            ]
        );
    }

    #[test]
    fn the_environment_overrides_the_built_in_defaults() {
        let settings = Settings::defaults(|name| {
            Some(
                match name {
                    "DICTATE_SHORTCUT" => "ctrl+shift+d",
                    "READ_ALOUD_SHORTCUT" => "alt+F10",
                    "VOICE_HOST" => "cosmiccrisp",
                    "VOICE_STT_PORT" => " 9000 ",
                    "VOICE_TTS_PORT" => "9001",
                    "VOICE_STT_LANGUAGE" => "AUTO",
                    _ => return None,
                }
                .into(),
            )
        });
        assert_eq!(
            settings.to_input(),
            SettingsInput {
                dictate_shortcut: Some("Ctrl+Shift+D".into()),
                read_aloud_shortcut: Some("Alt+F10".into()),
                voice_host: Some("cosmiccrisp".into()),
                stt_port: Some(9_000),
                tts_port: Some(9_001),
                stt_language: Some("auto".into()),
                ..defaults().to_input()
            }
        );
    }

    #[test]
    fn blank_or_invalid_environment_values_are_ignored() {
        let settings = Settings::defaults(|name| {
            Some(
                match name {
                    "DICTATE_SHORTCUT" => "  ",
                    "VOICE_HOST" => "",
                    "VOICE_STT_PORT" => "port",
                    "VOICE_TTS_PORT" => "70000",
                    "VOICE_STT_LANGUAGE" => "english",
                    _ => return None,
                }
                .into(),
            )
        });
        assert_eq!(settings, defaults());
    }

    #[test]
    fn a_valid_change_replaces_only_the_given_fields() {
        let input = SettingsInput {
            voice_host: Some("  cosmiccrisp ".into()),
            stt_port: Some(1),
            tts_port: Some(65_535),
            reduce_motion: Some(true),
            show_overlay: Some(false),
            start_at_sign_in: Some(true),
            ..SettingsInput::default()
        };
        assert_eq!(
            defaults().with(&input).map(|changed| changed.to_input()),
            Ok(SettingsInput {
                voice_host: Some("cosmiccrisp".into()),
                ..input.clone()
            }
            .or(&defaults().to_input()))
        );
    }

    #[test]
    fn an_empty_change_keeps_everything() {
        assert_eq!(defaults().with(&SettingsInput::default()), Ok(defaults()));
    }

    #[test]
    fn every_refused_field_is_reported() {
        let input = SettingsInput {
            dictate_shortcut: Some("win+l".into()),
            read_aloud_shortcut: Some("ctrl".into()),
            voice_host: Some(" ".into()),
            stt_port: Some(0),
            tts_port: Some(65_536),
            stt_language: Some("en-US".into()),
            ..SettingsInput::default()
        };
        assert_eq!(
            defaults().with(&input),
            Err(vec![
                error(Field::DICTATE_SHORTCUT, "Windows reserves Win+L."),
                error(
                    Field::READ_ALOUD_SHORTCUT,
                    "Add a key to go with the modifiers."
                ),
                error(
                    Field::VOICE_HOST,
                    "Enter the name or address of the computer running the voice servers."
                ),
                error(Field::STT_PORT, "Use a port from 1 to 65535."),
                error(Field::TTS_PORT, "Use a port from 1 to 65535."),
                error(
                    Field::STT_LANGUAGE,
                    "Use auto, or a language code such as en."
                ),
            ])
        );
    }

    #[test]
    fn negative_ports_are_refused() {
        let input = SettingsInput {
            stt_port: Some(-1),
            ..SettingsInput::default()
        };
        assert_eq!(
            defaults().with(&input),
            Err(vec![error(Field::STT_PORT, "Use a port from 1 to 65535.")])
        );
    }

    #[test]
    fn host_names_allow_names_and_addresses() {
        for name in [
            "cosmiccrisp",
            "box.tail1234.ts.net",
            "192.168.1.20",
            "[fe80::1%eth0]",
            "my_host",
        ] {
            assert_eq!(host(name), Ok(name.into()), "{name}");
        }
    }

    #[test]
    fn host_names_refuse_spaces_symbols_and_great_length() {
        assert_eq!(
            host("my box"),
            Err("A host name can't contain spaces.".into())
        );
        assert_eq!(host("box/path"), Err("A host name can't contain /.".into()));
        assert_eq!(host(&"a".repeat(253)), Ok("a".repeat(253)));
        assert_eq!(
            host(&"a".repeat(254)),
            Err("Use a host name of at most 253 characters.".into())
        );
    }

    #[test]
    fn languages_are_auto_or_two_or_three_letters() {
        assert_eq!(language(" EN "), Ok("en".into()));
        assert_eq!(language("haw"), Ok("haw".into()));
        assert_eq!(language("Auto"), Ok("auto".into()));
        for bad in ["e", "engl", "e1", "", "en-us"] {
            assert!(language(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_two_shortcuts_must_differ() {
        let input = SettingsInput {
            read_aloud_shortcut: Some("f9".into()),
            ..SettingsInput::default()
        };
        assert_eq!(
            defaults().with(&input),
            Err(vec![error(
                Field::READ_ALOUD_SHORTCUT,
                "Already used by Dictate."
            )])
        );
    }

    #[test]
    fn repairing_keeps_valid_fields_and_resets_refused_ones() {
        let input = SettingsInput {
            voice_host: Some("cosmiccrisp".into()),
            stt_port: Some(0),
            ..SettingsInput::default()
        };
        let (settings, errors) = defaults().repaired(&input);
        assert_eq!(settings.voice_host, "cosmiccrisp");
        assert_eq!(settings.stt_port, 10_300);
        assert_eq!(
            errors,
            [error(Field::STT_PORT, "Use a port from 1 to 65535.")]
        );
    }

    #[test]
    fn repairing_a_shortcut_clash_restores_both_shortcuts() {
        let input = SettingsInput {
            dictate_shortcut: Some("Ctrl+Alt+R".into()),
            read_aloud_shortcut: Some("ctrl+alt+r".into()),
            ..SettingsInput::default()
        };
        let (settings, errors) = defaults().repaired(&input);
        assert_eq!(settings.dictate_shortcut.to_string(), "F9");
        assert_eq!(settings.read_aloud_shortcut.to_string(), "Ctrl+Alt+R");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn stored_values_read_back_the_same() {
        let settings = defaults()
            .with(&SettingsInput {
                dictate_shortcut: Some("ctrl+space".into()),
                voice_host: Some("box".into()),
                start_at_sign_in: Some(true),
                ..SettingsInput::default()
            })
            .unwrap();
        let input = settings.to_input();
        assert_eq!(
            (
                input.dictate_shortcut.as_deref(),
                input.voice_host.as_deref()
            ),
            (Some("Ctrl+Space"), Some("box"))
        );
        assert_eq!(Settings::defaults(none).with(&input), Ok(settings));
    }

    #[test]
    fn fields_have_window_names() {
        assert_eq!(
            [
                Field::DICTATE_SHORTCUT,
                Field::READ_ALOUD_SHORTCUT,
                Field::VOICE_HOST,
                Field::STT_PORT,
                Field::TTS_PORT,
                Field::STT_LANGUAGE,
                Field::REDUCE_MOTION,
                Field::SHOW_OVERLAY,
                Field::START_AT_SIGN_IN,
            ]
            .map(Field::id),
            [
                "dictateShortcut",
                "readAloudShortcut",
                "voiceHost",
                "sttPort",
                "ttsPort",
                "sttLanguage",
                "reduceMotion",
                "showOverlay",
                "startAtSignIn",
            ]
        );
        assert_eq!(error(Field::STT_PORT, "bad").to_string(), "sttPort: bad");
    }
}
