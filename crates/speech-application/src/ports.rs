//! What the read-aloud and dictation use cases need from the outside world.

use speech_domain::{AudioClip, Voice, VoiceChoice};
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

/// Why recorded speech could not be turned into text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TranscriptionError {
    /// The speech recognition server could not be reached.
    #[error("can't reach the speech recognition server at {address}: {reason}")]
    Unreachable {
        /// Host and port that were tried.
        address: String,
        /// What the network reported.
        reason: String,
    },
    /// The speech recognition server answered with an error.
    #[error("the speech recognition server reported an error: {0}")]
    Server(String),
    /// The speech recognition server's answer did not follow the protocol.
    #[error("the speech recognition server sent something unexpected: {0}")]
    Protocol(String),
}

/// Why the microphone could not record.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("can't record from the microphone: {0}")]
pub struct CaptureError(pub String);

/// Turns recorded speech into text.
pub trait SpeechToText: Send + Sync {
    /// What was said in `speech`, as plain text. Empty when nothing was recognized.
    ///
    /// # Errors
    ///
    /// Any [`TranscriptionError`].
    fn transcribe(&self, speech: &AudioClip) -> Result<String, TranscriptionError>;
}

/// Records from the user's microphone between [`AudioInput::start`] and [`AudioInput::stop`].
pub trait AudioInput: Send + Sync {
    /// Opens the microphone and starts recording, dropping any recording still open.
    ///
    /// # Errors
    ///
    /// [`CaptureError`] if no input device can be opened.
    fn start(&self) -> Result<(), CaptureError>;

    /// Stops recording, closes the microphone, and returns what was recorded.
    ///
    /// # Errors
    ///
    /// [`CaptureError`] if nothing was being recorded or the recording is unusable.
    fn stop(&self) -> Result<AudioClip, CaptureError>;
}

/// Turns text into speech.
pub trait SpeechSynthesizer: Send + Sync {
    /// Audio for one sentence.
    ///
    /// # Errors
    ///
    /// Any [`SynthesisError`].
    fn synthesize(&self, sentence: &str) -> Result<AudioClip, SynthesisError>;

    /// Audio for one sentence in `voice`. A synthesizer with a single voice may ignore the
    /// choice, which is what the default does.
    ///
    /// # Errors
    ///
    /// Any [`SynthesisError`].
    fn synthesize_in(
        &self,
        sentence: &str,
        _voice: &VoiceChoice,
    ) -> Result<AudioClip, SynthesisError> {
        self.synthesize(sentence)
    }

    /// The voices this synthesizer offers; empty when there is nothing to choose from.
    ///
    /// # Errors
    ///
    /// Any [`SynthesisError`].
    fn voices(&self) -> Result<Vec<Voice>, SynthesisError>;
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

    struct OneVoice;

    impl SpeechSynthesizer for OneVoice {
        fn synthesize(&self, sentence: &str) -> Result<AudioClip, SynthesisError> {
            if sentence == "fail" {
                return Err(SynthesisError::Server("no voice".into()));
            }
            Ok(AudioClip::from_samples(1_000, vec![0.5; sentence.len()]).unwrap())
        }

        fn voices(&self) -> Result<Vec<Voice>, SynthesisError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn a_synthesizer_without_voices_speaks_in_its_only_voice() {
        let choice = VoiceChoice::parse(Some("en_US-amy-medium"), None).unwrap();
        let clip = OneVoice.synthesize_in("abc", &choice).unwrap();
        assert_eq!(clip.samples().len(), 3);
        assert_eq!(
            clip.samples(),
            OneVoice.synthesize("abc").unwrap().samples()
        );
    }

    #[test]
    fn a_failure_in_the_only_voice_is_passed_on() {
        let choice = VoiceChoice::parse(Some("en_US-amy-medium"), None).unwrap();
        assert_eq!(
            OneVoice.synthesize_in("fail", &choice).unwrap_err(),
            SynthesisError::Server("no voice".into())
        );
    }

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
        assert_eq!(
            CaptureError("no microphone".into()).to_string(),
            "can't record from the microphone: no microphone"
        );
    }

    #[test]
    fn transcription_errors_are_readable() {
        let unreachable = TranscriptionError::Unreachable {
            address: "voice-box:10300".into(),
            reason: "connection refused".into(),
        };
        assert_eq!(
            unreachable.to_string(),
            "can't reach the speech recognition server at voice-box:10300: connection refused"
        );
        assert_eq!(
            TranscriptionError::Server("no model".into()).to_string(),
            "the speech recognition server reported an error: no model"
        );
        assert_eq!(
            TranscriptionError::Protocol("bad header".into()).to_string(),
            "the speech recognition server sent something unexpected: bad header"
        );
    }
}
