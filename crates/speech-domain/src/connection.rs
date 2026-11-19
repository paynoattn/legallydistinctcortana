use std::fmt;

/// How the voice servers on the configured host are doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// Not checked yet.
    Checking,
    /// Speech recognition and the voice both answer.
    Connected,
    /// One of the two answers.
    Partial,
    /// Neither answers.
    Unreachable,
}

impl ConnectionKind {
    /// Stable lowercase name, for the UI.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Checking => "checking",
            Self::Connected => "connected",
            Self::Partial => "partial",
            Self::Unreachable => "unreachable",
        }
    }
}

/// The voice servers' state, with one sentence that says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionStatus {
    /// Which of the four states.
    pub kind: ConnectionKind,
    /// For the tray tooltip and the settings window.
    pub summary: String,
}

impl ConnectionStatus {
    /// Before the first check of `host`.
    #[must_use]
    pub fn checking(host: &str) -> Self {
        Self {
            kind: ConnectionKind::Checking,
            summary: format!("Checking {host}…"),
        }
    }

    /// The result of checking `host`: whether speech recognition answered, and whether
    /// the voice (text to speech) answered.
    #[must_use]
    pub fn checked(host: &str, recognition: bool, voice: bool) -> Self {
        let (kind, summary) = match (recognition, voice) {
            (true, true) => (ConnectionKind::Connected, format!("Connected to {host}.")),
            (true, false) => (
                ConnectionKind::Partial,
                format!("Speech recognition on {host} answers, but the voice server doesn't."),
            ),
            (false, true) => (
                ConnectionKind::Partial,
                format!("The voice server on {host} answers, but speech recognition doesn't."),
            ),
            (false, false) => (ConnectionKind::Unreachable, format!("Can't reach {host}.")),
        };
        Self { kind, summary }
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_servers_answering_is_connected() {
        let status = ConnectionStatus::checked("cosmiccrisp", true, true);
        assert_eq!(status.kind, ConnectionKind::Connected);
        assert_eq!(status.to_string(), "Connected to cosmiccrisp.");
    }

    #[test]
    fn one_server_answering_is_partial_and_says_which() {
        let recognition = ConnectionStatus::checked("box", true, false);
        assert_eq!(recognition.kind, ConnectionKind::Partial);
        assert_eq!(
            recognition.summary,
            "Speech recognition on box answers, but the voice server doesn't."
        );
        let voice = ConnectionStatus::checked("box", false, true);
        assert_eq!(voice.kind, ConnectionKind::Partial);
        assert_eq!(
            voice.summary,
            "The voice server on box answers, but speech recognition doesn't."
        );
    }

    #[test]
    fn no_server_answering_is_unreachable() {
        let status = ConnectionStatus::checked("box", false, false);
        assert_eq!(status.kind, ConnectionKind::Unreachable);
        assert_eq!(status.summary, "Can't reach box.");
    }

    #[test]
    fn before_a_check_it_says_it_is_checking() {
        let status = ConnectionStatus::checking("box");
        assert_eq!(status.kind, ConnectionKind::Checking);
        assert_eq!(status.summary, "Checking box…");
    }

    #[test]
    fn kinds_have_stable_ids() {
        assert_eq!(
            [
                ConnectionKind::Checking,
                ConnectionKind::Connected,
                ConnectionKind::Partial,
                ConnectionKind::Unreachable,
            ]
            .map(ConnectionKind::id),
            ["checking", "connected", "partial", "unreachable"]
        );
    }
}
