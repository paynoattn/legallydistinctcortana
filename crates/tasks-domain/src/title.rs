use thiserror::Error;

/// Longest allowed title, counted in Unicode scalar values after trimming.
pub const MAX_TITLE_CHARS: usize = 200;

/// Why a raw string is not a valid [`TaskTitle`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TitleError {
    /// Nothing left after trimming whitespace.
    #[error("title must not be empty")]
    Empty,
    /// Longer than [`MAX_TITLE_CHARS`] after trimming.
    #[error("title must be at most {MAX_TITLE_CHARS} characters, got {length}")]
    TooLong {
        /// Character count of the trimmed input.
        length: usize,
    },
}

/// A trimmed task title of 1 to [`MAX_TITLE_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTitle(String);

impl TaskTitle {
    /// Trims `raw` and validates its length.
    ///
    /// # Errors
    ///
    /// [`TitleError::Empty`] if only whitespace remains, [`TitleError::TooLong`] if
    /// more than [`MAX_TITLE_CHARS`] characters remain.
    pub fn parse(raw: &str) -> Result<Self, TitleError> {
        let trimmed = raw.trim();
        let length = trimmed.chars().count();
        if length == 0 {
            return Err(TitleError::Empty);
        }
        if length > MAX_TITLE_CHARS {
            return Err(TitleError::TooLong { length });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The validated title text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        let title = TaskTitle::parse("  buy milk \n").unwrap();
        assert_eq!(title.as_str(), "buy milk");
    }

    #[test]
    fn rejects_empty_and_whitespace_only() {
        assert_eq!(TaskTitle::parse(""), Err(TitleError::Empty));
        assert_eq!(TaskTitle::parse(" \t\n"), Err(TitleError::Empty));
    }

    #[test]
    fn accepts_one_character() {
        assert_eq!(TaskTitle::parse("a").unwrap().as_str(), "a");
    }

    #[test]
    fn accepts_exactly_max_characters() {
        let raw = "a".repeat(MAX_TITLE_CHARS);
        assert_eq!(TaskTitle::parse(&raw).unwrap().as_str(), raw);
    }

    #[test]
    fn rejects_one_over_max_characters() {
        let raw = "a".repeat(MAX_TITLE_CHARS + 1);
        assert_eq!(
            TaskTitle::parse(&raw),
            Err(TitleError::TooLong {
                length: MAX_TITLE_CHARS + 1
            })
        );
    }

    #[test]
    fn counts_characters_not_bytes() {
        let raw = "é".repeat(MAX_TITLE_CHARS);
        assert!(TaskTitle::parse(&raw).is_ok());
    }

    #[test]
    fn length_limit_applies_after_trimming() {
        let raw = format!("  {}  ", "a".repeat(MAX_TITLE_CHARS));
        assert!(TaskTitle::parse(&raw).is_ok());
    }

    #[test]
    fn error_messages_are_readable() {
        assert_eq!(TitleError::Empty.to_string(), "title must not be empty");
        assert_eq!(
            TitleError::TooLong { length: 201 }.to_string(),
            "title must be at most 200 characters, got 201"
        );
    }
}
