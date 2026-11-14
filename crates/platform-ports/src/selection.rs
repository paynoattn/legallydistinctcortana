use thiserror::Error;

/// Why the selected text could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SelectionError {
    /// The focused app has no selection, or did not copy it.
    #[error("nothing is selected")]
    NothingSelected,
    /// The clipboard or input system failed.
    #[error("could not read the selection: {0}")]
    Unavailable(String),
}

/// Reads the text currently selected in the app the user is working in.
pub trait SelectionReader: Send + Sync {
    /// The selected text, exactly as the app provides it.
    ///
    /// # Errors
    ///
    /// [`SelectionError::NothingSelected`] or [`SelectionError::Unavailable`].
    fn read_selection(&self) -> Result<String, SelectionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_readable() {
        assert_eq!(
            SelectionError::NothingSelected.to_string(),
            "nothing is selected"
        );
        assert_eq!(
            SelectionError::Unavailable("clipboard busy".into()).to_string(),
            "could not read the selection: clipboard busy"
        );
    }
}
