use std::time::Duration;

use thiserror::Error;

/// Host used when `VOICE_HOST` is not set.
pub const DEFAULT_HOST: &str = "localhost";

/// Port of the Wyoming Piper container when `VOICE_TTS_PORT` is not set.
pub const DEFAULT_TTS_PORT: u16 = 10_200;

/// Port of the Wyoming Whisper container when `VOICE_STT_PORT` is not set.
pub const DEFAULT_STT_PORT: u16 = 10_300;

/// Language asked of Whisper when `VOICE_STT_LANGUAGE` is not set, as in `voice.py`.
pub const DEFAULT_STT_LANGUAGE: &str = "en";

/// `VOICE_STT_LANGUAGE` value that lets the server detect the language.
pub const AUTO_LANGUAGE: &str = "auto";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(30);
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(60);

/// Why the voice server settings are invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// A port variable is not a port number.
    #[error("{variable} must be a port number from 1 to 65535, got {value:?}")]
    InvalidPort {
        /// The environment variable.
        variable: &'static str,
        /// What it was set to.
        value: String,
    },
}

/// Where the Wyoming text-to-speech server is, and which voice to ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WyomingConfig {
    /// Host name or address (a Tailscale name works).
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Voice name, for example `en_US-amy-medium`. `None` uses the server's default.
    pub voice: Option<String>,
    /// How long to wait for the connection.
    pub connect_timeout: Duration,
    /// How long to wait for each read or write once connected.
    pub io_timeout: Duration,
}

impl WyomingConfig {
    /// Settings from `lookup` (normally the process environment): `VOICE_HOST`,
    /// `VOICE_TTS_PORT`, `VOICE_TTS_VOICE`. Blank values count as unset.
    ///
    /// # Errors
    ///
    /// [`ConfigError::InvalidPort`].
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let setting = settings(lookup);
        Ok(Self {
            host: host(&setting),
            port: port(&setting, "VOICE_TTS_PORT", DEFAULT_TTS_PORT)?,
            voice: setting("VOICE_TTS_VOICE"),
            connect_timeout: CONNECT_TIMEOUT,
            io_timeout: IO_TIMEOUT,
        })
    }

    /// `host:port`, for messages.
    #[must_use]
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Where the Wyoming speech-to-text server is, and which language to ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WyomingSttConfig {
    /// Host name or address (a Tailscale name works).
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Language code, for example `en`. `None` lets the server detect it.
    pub language: Option<String>,
    /// How long to wait for the connection.
    pub connect_timeout: Duration,
    /// How long to wait for the transcript once the audio is sent.
    pub io_timeout: Duration,
}

impl WyomingSttConfig {
    /// Settings from `lookup` (normally the process environment): `VOICE_HOST`,
    /// `VOICE_STT_PORT`, `VOICE_STT_LANGUAGE` (`auto` lets the server detect it). Blank
    /// values count as unset.
    ///
    /// # Errors
    ///
    /// [`ConfigError::InvalidPort`].
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let setting = settings(lookup);
        let language =
            setting("VOICE_STT_LANGUAGE").unwrap_or_else(|| DEFAULT_STT_LANGUAGE.to_owned());
        Ok(Self {
            host: host(&setting),
            port: port(&setting, "VOICE_STT_PORT", DEFAULT_STT_PORT)?,
            language: Some(language).filter(|code| !code.eq_ignore_ascii_case(AUTO_LANGUAGE)),
            connect_timeout: CONNECT_TIMEOUT,
            io_timeout: TRANSCRIBE_TIMEOUT,
        })
    }

    /// `host:port`, for messages.
    #[must_use]
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn settings(lookup: impl Fn(&str) -> Option<String>) -> impl Fn(&str) -> Option<String> {
    move |key| {
        lookup(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }
}

fn host(setting: &impl Fn(&str) -> Option<String>) -> String {
    setting("VOICE_HOST").unwrap_or_else(|| DEFAULT_HOST.to_owned())
}

fn port(
    setting: &impl Fn(&str) -> Option<String>,
    variable: &'static str,
    default: u16,
) -> Result<u16, ConfigError> {
    setting(variable).map_or(Ok(default), |value| {
        value
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(ConfigError::InvalidPort { variable, value })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let env: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |key| env.get(key).cloned()
    }

    fn config(pairs: &[(&str, &str)]) -> Result<WyomingConfig, ConfigError> {
        WyomingConfig::from_lookup(lookup(pairs))
    }

    fn stt(pairs: &[(&str, &str)]) -> Result<WyomingSttConfig, ConfigError> {
        WyomingSttConfig::from_lookup(lookup(pairs))
    }

    #[test]
    fn defaults_to_a_local_piper_with_its_default_voice() {
        let defaults = config(&[]).unwrap();
        assert_eq!(defaults.host, "localhost");
        assert_eq!(defaults.port, 10_200);
        assert_eq!(defaults.voice, None);
        assert_eq!(defaults.address(), "localhost:10200");
    }

    #[test]
    fn waits_three_seconds_to_connect_and_thirty_per_read() {
        let defaults = config(&[]).unwrap();
        assert_eq!(defaults.connect_timeout, Duration::from_secs(3));
        assert_eq!(defaults.io_timeout, Duration::from_secs(30));
    }

    #[test]
    fn reads_the_same_variables_as_voice_py() {
        let custom = config(&[
            ("VOICE_HOST", " voice-box "),
            ("VOICE_TTS_PORT", "10201"),
            ("VOICE_TTS_VOICE", "en_US-amy-medium"),
        ])
        .unwrap();
        assert_eq!(custom.address(), "voice-box:10201");
        assert_eq!(custom.voice.as_deref(), Some("en_US-amy-medium"));
    }

    #[test]
    fn blank_values_count_as_unset() {
        let blank = config(&[("VOICE_HOST", "  "), ("VOICE_TTS_VOICE", "")]).unwrap();
        assert_eq!(blank.host, "localhost");
        assert_eq!(blank.voice, None);
    }

    #[test]
    fn rejects_ports_that_are_not_port_numbers() {
        for bad in ["0", "70000", "piper"] {
            assert_eq!(
                config(&[("VOICE_TTS_PORT", bad)]),
                Err(ConfigError::InvalidPort {
                    variable: "VOICE_TTS_PORT",
                    value: bad.into()
                })
            );
        }
        assert_eq!(
            config(&[("VOICE_TTS_PORT", "x")]).unwrap_err().to_string(),
            "VOICE_TTS_PORT must be a port number from 1 to 65535, got \"x\""
        );
    }

    #[test]
    fn speech_recognition_defaults_to_a_local_whisper_in_english() {
        let defaults = stt(&[]).unwrap();
        assert_eq!(defaults.address(), "localhost:10300");
        assert_eq!(defaults.language.as_deref(), Some("en"));
        assert_eq!(defaults.connect_timeout, Duration::from_secs(3));
        assert_eq!(defaults.io_timeout, Duration::from_secs(60));
    }

    #[test]
    fn speech_recognition_reads_the_same_variables_as_voice_py() {
        let custom = stt(&[
            ("VOICE_HOST", "voice-box"),
            ("VOICE_STT_PORT", " 10301 "),
            ("VOICE_STT_LANGUAGE", "de"),
            ("VOICE_TTS_PORT", "10201"),
        ])
        .unwrap();
        assert_eq!(custom.address(), "voice-box:10301");
        assert_eq!(custom.language.as_deref(), Some("de"));
    }

    #[test]
    fn auto_lets_the_server_detect_the_language() {
        for auto in ["auto", "AUTO"] {
            assert_eq!(stt(&[("VOICE_STT_LANGUAGE", auto)]).unwrap().language, None);
        }
        assert_eq!(
            stt(&[("VOICE_STT_LANGUAGE", " ")])
                .unwrap()
                .language
                .as_deref(),
            Some("en")
        );
    }

    #[test]
    fn speech_recognition_rejects_ports_that_are_not_port_numbers() {
        assert_eq!(
            stt(&[("VOICE_STT_PORT", "0")]),
            Err(ConfigError::InvalidPort {
                variable: "VOICE_STT_PORT",
                value: "0".into()
            })
        );
        assert_eq!(
            stt(&[("VOICE_STT_PORT", "whisper")])
                .unwrap_err()
                .to_string(),
            "VOICE_STT_PORT must be a port number from 1 to 65535, got \"whisper\""
        );
    }
}
