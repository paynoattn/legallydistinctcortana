use std::time::Duration;

use thiserror::Error;

/// Host used when `VOICE_HOST` is not set.
pub const DEFAULT_HOST: &str = "localhost";

/// Port of the Wyoming Piper container when `VOICE_TTS_PORT` is not set.
pub const DEFAULT_TTS_PORT: u16 = 10_200;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Why the voice server settings are invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// `VOICE_TTS_PORT` is not a port number.
    #[error("VOICE_TTS_PORT must be a port number from 1 to 65535, got {0:?}")]
    InvalidPort(String),
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
        let setting = |key: &str| {
            lookup(key)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let port = setting("VOICE_TTS_PORT").map_or(Ok(DEFAULT_TTS_PORT), parse_port)?;
        Ok(Self {
            host: setting("VOICE_HOST").unwrap_or_else(|| DEFAULT_HOST.to_owned()),
            port,
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

fn parse_port(raw: String) -> Result<u16, ConfigError> {
    raw.parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(ConfigError::InvalidPort(raw))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(pairs: &[(&str, &str)]) -> Result<WyomingConfig, ConfigError> {
        let env: HashMap<&str, &str> = pairs.iter().copied().collect();
        WyomingConfig::from_lookup(|key| env.get(key).map(|value| (*value).to_owned()))
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
                Err(ConfigError::InvalidPort(bad.into()))
            );
        }
        assert_eq!(
            ConfigError::InvalidPort("x".into()).to_string(),
            "VOICE_TTS_PORT must be a port number from 1 to 65535, got \"x\""
        );
    }
}
