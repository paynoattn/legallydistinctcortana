use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// How the assistant should sound when it speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    /// Plain and even.
    Neutral,
    /// Slower and softer.
    Calm,
    /// Brighter and a little quicker.
    Cheerful,
    /// Crisp and raised, to get attention.
    Alert,
    /// Fast and loud: act now.
    Urgent,
    /// Clipped and forceful.
    Angry,
    /// Angry turned up to theatrical: louder, faster, clipped.
    Mad,
    /// Quiet and slow.
    Whisper,
    /// Flat and unimpressed.
    Deadpan,
}

impl Mood {
    /// Every mood, in menu order.
    pub const ALL: [Self; 9] = [
        Self::Neutral,
        Self::Calm,
        Self::Cheerful,
        Self::Alert,
        Self::Urgent,
        Self::Angry,
        Self::Mad,
        Self::Whisper,
        Self::Deadpan,
    ];

    /// Stable lowercase id used over IPC, in settings, and by the browser extension.
    #[must_use]
    pub fn id(self) -> &'static str {
        IDS.iter()
            .find_map(|(mood, id)| (*mood == self).then_some(*id))
            .unwrap_or_default()
    }
}

const IDS: [(Mood, &str); 9] = [
    (Mood::Neutral, "neutral"),
    (Mood::Calm, "calm"),
    (Mood::Cheerful, "cheerful"),
    (Mood::Alert, "alert"),
    (Mood::Urgent, "urgent"),
    (Mood::Angry, "angry"),
    (Mood::Mad, "mad"),
    (Mood::Whisper, "whisper"),
    (Mood::Deadpan, "deadpan"),
];

impl fmt::Display for Mood {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A mood id that is not one of [`Mood::ALL`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMood {
    id: String,
}

impl UnknownMood {
    /// The id that did not match any mood.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for UnknownMood {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown mood {:?}", self.id)
    }
}

impl Error for UnknownMood {}

impl FromStr for Mood {
    type Err = UnknownMood;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mood| mood.id() == raw)
            .ok_or_else(|| UnknownMood { id: raw.to_owned() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_round_trips() {
        for mood in Mood::ALL {
            assert_eq!(mood.id().parse::<Mood>(), Ok(mood));
            assert_eq!(mood.to_string(), mood.id());
        }
    }

    #[test]
    fn ids_are_the_documented_lowercase_names() {
        let ids: Vec<&str> = Mood::ALL.iter().map(|mood| mood.id()).collect();
        assert_eq!(
            ids,
            [
                "neutral", "calm", "cheerful", "alert", "urgent", "angry", "mad", "whisper",
                "deadpan"
            ]
        );
    }

    #[test]
    fn unknown_ids_are_rejected_with_the_offending_id() {
        let error = "Calm".parse::<Mood>().unwrap_err();
        assert_eq!(error.id(), "Calm");
        assert_eq!(error.to_string(), "unknown mood \"Calm\"");
    }
}
