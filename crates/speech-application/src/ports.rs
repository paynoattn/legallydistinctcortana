//! What the read-aloud use cases need from the outside world.

use speech_domain::AudioClip;
use thiserror::Error;

use crate::CancelFlag;

/// Why a sentence could not be synthesized.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SynthesisError {
    /// The voice server could not be reached.
    #[error("can't reach the voice server at {address}: {reason}")]
    Unreachable {
        /// Host and port that were tried.
        address: String,
        /// What the network reported.
        reason: String,
    },
    /// The voice server answered with an error.
    #[error("the voice server reported an error: {0}")]
    Server(String),
    /// The voice server's answer did not follow the protocol.
    #[error("the voice server sent something unexpected: {0}")]
    Protocol(String),
}

/// Why audio could not be played.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("can't play audio: {0}")]
pub struct PlaybackError(pub String);

/// Turns text into speech.
pub trait SpeechSynthesizer: Send + Sync {
    /// Audio for one sentence.
    ///
    /// # Errors
    ///
    /// Any [`SynthesisError`].
    fn synthesize(&self, sentence: &str) -> Result<AudioClip, SynthesisError>;
}

/// Plays audio on the user's speakers, in the order it is queued.
pub trait AudioOutput: Send + Sync {
    /// Queues `clip` after anything already queued and returns without waiting.
    ///
    /// # Errors
    ///
    /// [`PlaybackError`] if no output device can be opened.
    fn enqueue(&self, clip: AudioClip) -> Result<(), PlaybackError>;

    /// Blocks until everything queued has played, or `cancel` is set.
    fn wait_until_idle(&self, cancel: &CancelFlag);

    /// Drops everything queued and goes silent at once.
    fn stop(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_readable() {
        let unreachable = SynthesisError::Unreachable {
            address: "voice-box:10200".into(),
            reason: "connection refused".into(),
        };
        assert_eq!(
            unreachable.to_string(),
            "can't reach the voice server at voice-box:10200: connection refused"
        );
        assert_eq!(
            SynthesisError::Server("no voice".into()).to_string(),
            "the voice server reported an error: no voice"
        );
        assert_eq!(
            SynthesisError::Protocol("bad header".into()).to_string(),
            "the voice server sent something unexpected: bad header"
        );
        assert_eq!(
            PlaybackError("no device".into()).to_string(),
            "can't play audio: no device"
        );
    }
}
