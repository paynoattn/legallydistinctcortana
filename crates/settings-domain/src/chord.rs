use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Keys named by their `KeyboardEvent.code`, besides letters, digits, and F1 to F24.
const NAMED_KEYS: [&str; 52] = [
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "Equal",
    "Minus",
    "Period",
    "Quote",
    "Semicolon",
    "Slash",
    "Backspace",
    "Enter",
    "Space",
    "Tab",
    "Delete",
    "End",
    "Home",
    "Insert",
    "PageDown",
    "PageUp",
    "PrintScreen",
    "ScrollLock",
    "Pause",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadDecimal",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadMultiply",
    "NumpadSubtract",
    "Escape",
    "CapsLock",
    "NumLock",
    "MediaPlayPause",
    "MediaStop",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "AudioVolumeMute",
];

/// Keys that may be a shortcut on their own: pressing them alone does nothing else useful
/// while typing.
const STANDALONE_KEYS: [&str; 4] = ["Pause", "ScrollLock", "Insert", "PrintScreen"];

/// Chords Windows keeps for itself, written as [`Chord`] displays them.
const RESERVED: [&str; 14] = [
    "Ctrl+Alt+Delete",
    "Ctrl+Shift+Escape",
    "Ctrl+Escape",
    "Alt+Tab",
    "Alt+F4",
    "Alt+Escape",
    "Win+L",
    "Win+D",
    "Win+E",
    "Win+R",
    "Win+I",
    "Win+X",
    "Win+Tab",
    "Win+A",
];

/// Why text is not a usable shortcut. Messages are written for the settings window.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChordError {
    /// Nothing was entered.
    #[error("Press a key to use as the shortcut.")]
    Empty,
    /// Only modifiers, no key.
    #[error("Add a key to go with the modifiers.")]
    NoKey,
    /// More than one non-modifier key.
    #[error("Use one key, plus any of Ctrl, Alt, Shift and Win.")]
    TwoKeys,
    /// A key this app can't listen for.
    #[error("{0} can't be used in a shortcut.")]
    UnknownKey(String),
    /// A typing key without any modifier.
    #[error("Add Ctrl, Alt, Shift or Win: on its own, {0} would stop working for typing.")]
    NeedsModifier(String),
    /// A chord Windows keeps for itself.
    #[error("Windows reserves {0}.")]
    Reserved(String),
}

/// A keyboard shortcut: modifiers plus one key, such as `Ctrl+Alt+R` or `F9`.
///
/// Parsing accepts any case and the aliases `Control`, `Super`, and `Meta`. Display is
/// canonical: modifiers in the order Ctrl, Alt, Shift, Win, then the key as a letter, digit,
/// F-key, or `KeyboardEvent.code` name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Modifier {
    fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Self::Ctrl),
            "alt" => Some(Self::Alt),
            "shift" => Some(Self::Shift),
            "win" | "super" | "meta" => Some(Self::Win),
            _ => None,
        }
    }

    const fn shown(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
            Self::Win => "Win",
        }
    }

    const fn accelerator(self) -> &'static str {
        match self {
            Self::Ctrl => "ctrl",
            Self::Alt => "alt",
            Self::Shift => "shift",
            Self::Win => "super",
        }
    }
}

/// A keyboard shortcut: modifiers plus one key, such as `Ctrl+Alt+R` or `F9`.
///
/// Parsing accepts any case and the aliases `Control`, `Super`, and `Meta`. Display is
/// canonical: modifiers in the order Ctrl, Alt, Shift, Win, then the key as a letter, digit,
/// F-key, or `KeyboardEvent.code` name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chord {
    modifiers: BTreeSet<Modifier>,
    key: String,
}

impl Chord {
    pub(crate) fn default_dictate() -> Self {
        Self {
            modifiers: BTreeSet::new(),
            key: "F9".into(),
        }
    }

    pub(crate) fn default_read_aloud() -> Self {
        Self {
            modifiers: BTreeSet::from([Modifier::Ctrl, Modifier::Alt]),
            key: "R".into(),
        }
    }

    /// The chord in the form the global shortcut library parses (`Win` is `super`).
    #[must_use]
    pub fn accelerator(&self) -> String {
        self.joined(Modifier::accelerator)
    }

    fn joined(&self, name: fn(Modifier) -> &'static str) -> String {
        self.modifiers
            .iter()
            .map(|modifier| name(*modifier))
            .chain([self.key.as_str()])
            .collect::<Vec<_>>()
            .join("+")
    }

    fn checked(self) -> Result<Self, ChordError> {
        let shown = self.to_string();
        if RESERVED.contains(&shown.as_str()) {
            return Err(ChordError::Reserved(shown));
        }
        if !self.modifiers.is_empty()
            || is_function_key(&self.key)
            || STANDALONE_KEYS.contains(&self.key.as_str())
        {
            return Ok(self);
        }
        Err(ChordError::NeedsModifier(self.key))
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.joined(Modifier::shown))
    }
}

impl FromStr for Chord {
    type Err = ChordError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = text
            .split('+')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .collect();
        if tokens.is_empty() {
            return Err(ChordError::Empty);
        }
        let mut modifiers = BTreeSet::new();
        let mut key: Option<String> = None;
        for token in tokens {
            if let Some(modifier) = Modifier::parse(token) {
                modifiers.insert(modifier);
            } else if key.is_some() {
                return Err(ChordError::TwoKeys);
            } else {
                key = Some(key_name(token).ok_or_else(|| ChordError::UnknownKey(token.into()))?);
            }
        }
        let key = key.ok_or(ChordError::NoKey)?;
        Self { modifiers, key }.checked()
    }
}

fn key_name(token: &str) -> Option<String> {
    let upper = token.to_ascii_uppercase();
    let bare = upper
        .strip_prefix("KEY")
        .or_else(|| upper.strip_prefix("DIGIT"))
        .filter(|rest| rest.len() == 1)
        .unwrap_or(&upper);
    if bare.len() == 1 && bare.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(bare.to_owned());
    }
    if is_function_key(&upper) {
        return Some(upper);
    }
    NAMED_KEYS
        .iter()
        .find(|name| name.eq_ignore_ascii_case(token))
        .map(|name| (*name).to_owned())
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('F')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=24).contains(&number))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> Result<String, ChordError> {
        text.parse::<Chord>().map(|chord| chord.to_string())
    }

    #[test]
    fn modifiers_are_put_in_canonical_order_and_case() {
        assert_eq!(chord("alt+ctrl+r"), Ok("Ctrl+Alt+R".into()));
        assert_eq!(chord("Win + Shift + Space"), Ok("Shift+Win+Space".into()));
        assert_eq!(
            chord("Control+Super+Alt+Shift+k"),
            Ok("Ctrl+Alt+Shift+Win+K".into())
        );
        assert_eq!(chord("meta+F2"), Ok("Win+F2".into()));
    }

    #[test]
    fn letters_and_digits_may_be_written_as_codes() {
        assert_eq!(chord("Ctrl+KeyQ"), Ok("Ctrl+Q".into()));
        assert_eq!(chord("Ctrl+Digit7"), Ok("Ctrl+7".into()));
        assert_eq!(chord("Ctrl+7"), Ok("Ctrl+7".into()));
    }

    #[test]
    fn named_keys_use_their_code_spelling() {
        assert_eq!(chord("ctrl+backquote"), Ok("Ctrl+Backquote".into()));
        assert_eq!(chord("alt+NUMPAD5"), Ok("Alt+Numpad5".into()));
        assert_eq!(chord("Shift+arrowup"), Ok("Shift+ArrowUp".into()));
    }

    #[test]
    fn function_keys_work_alone_from_f1_to_f24() {
        assert_eq!(
            ["f9", "F1", "F24", "F25", "F0", "Fx"].map(chord),
            [
                Ok("F9".into()),
                Ok("F1".into()),
                Ok("F24".into()),
                Err(ChordError::UnknownKey("F25".into())),
                Err(ChordError::UnknownKey("F0".into())),
                Err(ChordError::UnknownKey("Fx".into())),
            ]
        );
    }

    #[test]
    fn a_few_non_typing_keys_work_alone() {
        for key in ["Pause", "ScrollLock", "Insert", "PrintScreen"] {
            assert_eq!(chord(key), Ok(key.into()));
        }
    }

    #[test]
    fn typing_keys_alone_are_refused() {
        assert_eq!(chord("r"), Err(ChordError::NeedsModifier("R".into())));
        assert_eq!(
            chord("Space"),
            Err(ChordError::NeedsModifier("Space".into()))
        );
        assert_eq!(
            ChordError::NeedsModifier("R".into()).to_string(),
            "Add Ctrl, Alt, Shift or Win: on its own, R would stop working for typing."
        );
    }

    #[test]
    fn any_modifier_makes_a_typing_key_usable() {
        for text in ["Ctrl+R", "Alt+R", "Shift+R", "Win+R+Ctrl"] {
            assert!(chord(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn malformed_chords_are_refused() {
        assert_eq!(
            [
                "",
                " + ",
                "Ctrl+Alt",
                "Ctrl+A+B",
                "Ctrl+Hyper",
                "Ctrl+ab",
                "Ctrl+!"
            ]
            .map(chord),
            [
                Err(ChordError::Empty),
                Err(ChordError::Empty),
                Err(ChordError::NoKey),
                Err(ChordError::TwoKeys),
                Err(ChordError::UnknownKey("Hyper".into())),
                Err(ChordError::UnknownKey("ab".into())),
                Err(ChordError::UnknownKey("!".into())),
            ]
        );
    }

    #[test]
    fn windows_shortcuts_are_reserved() {
        assert_eq!(chord("win+l"), Err(ChordError::Reserved("Win+L".into())));
        assert_eq!(
            chord("alt+ctrl+delete"),
            Err(ChordError::Reserved("Ctrl+Alt+Delete".into()))
        );
        assert_eq!(
            ChordError::Reserved("Win+L".into()).to_string(),
            "Windows reserves Win+L."
        );
        assert!(chord("Win+Shift+L").is_ok(), "only the exact chord");
    }

    #[test]
    fn the_accelerator_spells_win_as_super() {
        let chord: Chord = "Win+Ctrl+Shift+Alt+Numpad5".parse().unwrap();
        assert_eq!(chord.accelerator(), "ctrl+alt+shift+super+Numpad5");
        let bare: Chord = "f9".parse().unwrap();
        assert_eq!(bare.accelerator(), "F9");
    }

    #[test]
    fn messages_are_readable() {
        assert_eq!(
            ChordError::Empty.to_string(),
            "Press a key to use as the shortcut."
        );
        assert_eq!(
            ChordError::NoKey.to_string(),
            "Add a key to go with the modifiers."
        );
        assert_eq!(
            ChordError::TwoKeys.to_string(),
            "Use one key, plus any of Ctrl, Alt, Shift and Win."
        );
        assert_eq!(
            ChordError::UnknownKey("Hyper".into()).to_string(),
            "Hyper can't be used in a shortcut."
        );
    }
}
