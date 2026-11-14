/// A voice a synthesizer offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// The id the synthesizer knows it by, for example `en_US-amy-medium`.
    pub name: String,
    /// A human label, for example `amy (medium)`.
    pub description: String,
    /// Language codes as the synthesizer reports them, for example `en_US`.
    pub languages: Vec<String>,
    /// Speaker names for voices that hold several speakers; empty otherwise.
    pub speakers: Vec<String>,
}

/// The voice to read with and, for a multi-speaker voice, the speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceChoice {
    name: String,
    speaker: Option<String>,
}

impl VoiceChoice {
    /// A choice from raw settings. A blank or missing name means "the synthesizer's default
    /// voice" (`None`); a blank speaker means "the voice's default speaker".
    #[must_use]
    pub fn parse(name: Option<&str>, speaker: Option<&str>) -> Option<Self> {
        let name = name.map(str::trim).filter(|name| !name.is_empty())?;
        Some(Self {
            name: name.to_owned(),
            speaker: speaker
                .map(str::trim)
                .filter(|speaker| !speaker.is_empty())
                .map(str::to_owned),
        })
    }

    /// The voice id.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The speaker, if one was chosen.
    #[must_use]
    pub fn speaker(&self) -> Option<&str> {
        self.speaker.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_makes_a_choice_and_blanks_mean_the_default() {
        let choice = VoiceChoice::parse(Some(" en_US-amy-medium "), Some(" ")).unwrap();
        assert_eq!(choice.name(), "en_US-amy-medium");
        assert_eq!(choice.speaker(), None);
        assert_eq!(VoiceChoice::parse(Some("  "), Some("p1")), None);
        assert_eq!(VoiceChoice::parse(None, Some("p1")), None);
    }

    #[test]
    fn no_speaker_and_an_empty_speaker_both_mean_the_default_speaker() {
        let none = VoiceChoice::parse(Some("a"), None).unwrap();
        let empty = VoiceChoice::parse(Some("a"), Some("")).unwrap();
        assert_eq!(none, empty);
        assert_eq!(none.speaker(), None);
    }

    #[test]
    fn an_empty_name_is_no_choice() {
        assert_eq!(VoiceChoice::parse(Some(""), None), None);
        assert_eq!(VoiceChoice::parse(Some("\t\n"), Some("p1")), None);
    }

    #[test]
    fn a_choice_with_and_without_a_speaker_differ() {
        let plain = VoiceChoice::parse(Some("a"), None).unwrap();
        let spoken = VoiceChoice::parse(Some("a"), Some("p1")).unwrap();
        assert_ne!(plain, spoken);
        assert_eq!(spoken.name(), "a");
    }

    #[test]
    fn a_speaker_is_kept_trimmed() {
        let choice = VoiceChoice::parse(Some("en_US-libritts-high"), Some(" p239 ")).unwrap();
        assert_eq!(choice.speaker(), Some("p239"));
    }
}
