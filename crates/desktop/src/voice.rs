//! The speech services, built from the settings and rebuilt when the voice server changes.

use std::env;
use std::sync::{Arc, PoisonError, RwLock};

use platform_desktop::{
    ArboardClipboard, ClipboardSelectionReader, ClipboardTextInserter, EnigoKeyboard,
};
use settings_application::Settings;
use speech_application::{DictationService, ReadAloudService};
use speech_infrastructure::{
    ConfigError, MicrophoneInput, SpeakerOutput, WyomingConfig, WyomingSttConfig,
    WyomingSynthesizer, WyomingTranscriber,
};

pub(crate) struct Voice {
    dictation: RwLock<Arc<DictationService>>,
    read_aloud: RwLock<Arc<ReadAloudService>>,
}

impl Voice {
    pub(crate) fn new(settings: &Settings) -> Result<Self, ConfigError> {
        Ok(Self {
            dictation: RwLock::new(dictation(settings)?),
            read_aloud: RwLock::new(read_aloud(settings)?),
        })
    }

    pub(crate) fn dictation(&self) -> Arc<DictationService> {
        Arc::clone(
            &self
                .dictation
                .read()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    pub(crate) fn read_aloud(&self) -> Arc<ReadAloudService> {
        Arc::clone(
            &self
                .read_aloud
                .read()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    pub(crate) fn rebuild(&self, settings: &Settings) -> Result<(), ConfigError> {
        let dictation = dictation(settings)?;
        let read_aloud = read_aloud(settings)?;
        self.read_aloud().stop();
        *self
            .dictation
            .write()
            .unwrap_or_else(PoisonError::into_inner) = dictation;
        *self
            .read_aloud
            .write()
            .unwrap_or_else(PoisonError::into_inner) = read_aloud;
        Ok(())
    }
}

fn dictation(settings: &Settings) -> Result<Arc<DictationService>, ConfigError> {
    let config = WyomingSttConfig::from_lookup(lookup(settings))?;
    tracing::info!(server = %config.address(), "dictation uses this Wyoming speech recognition server");
    Ok(Arc::new(DictationService::new(
        Arc::new(MicrophoneInput::default()),
        Arc::new(WyomingTranscriber::new(config)),
        Arc::new(ClipboardTextInserter::new(ArboardClipboard, EnigoKeyboard)),
    )))
}

fn read_aloud(settings: &Settings) -> Result<Arc<ReadAloudService>, ConfigError> {
    let config = WyomingConfig::from_lookup(lookup(settings))?;
    tracing::info!(server = %config.address(), "read aloud uses this Wyoming voice server");
    Ok(Arc::new(ReadAloudService::new(
        Arc::new(ClipboardSelectionReader::new(
            ArboardClipboard,
            EnigoKeyboard,
        )),
        Arc::new(WyomingSynthesizer::new(config)),
        Arc::new(SpeakerOutput::default()),
    )))
}

fn lookup(settings: &Settings) -> impl Fn(&str) -> Option<String> {
    let settings = settings.clone();
    move |name| settings.variable(name).or_else(|| env::var(name).ok())
}
