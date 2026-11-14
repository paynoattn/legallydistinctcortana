use thiserror::Error;

/// Why text could not be put into the focused app.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("could not type into the focused app: {0}")]
pub struct InsertError(pub String);

/// Puts text at the cursor of the app the user is working in, as if they had typed it.
pub trait TextInserter: Send + Sync {
    /// Inserts `text` at the cursor of the focused app.
    ///
    /// # Errors
    ///
    /// [`InsertError`] when the clipboard or the input system fails.
    fn insert(&self, text: &str) -> Result<(), InsertError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_message_is_readable() {
        assert_eq!(
            InsertError("clipboard busy".into()).to_string(),
            "could not type into the focused app: clipboard busy"
        );
    }
}
