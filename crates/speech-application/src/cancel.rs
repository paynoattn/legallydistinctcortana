use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A shared "stop now" signal. Clones observe the same flag.
#[derive(Debug, Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// A flag that is not cancelled yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks everyone holding a clone to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether `other` is this flag or one of its clones.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Whether [`Self::cancel`] has been called on this flag or a clone.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_means_this_flag_or_a_clone_of_it() {
        let flag = CancelFlag::new();
        assert!(flag.same(&flag.clone()));
        assert!(!flag.same(&CancelFlag::new()));
    }

    #[test]
    fn clones_share_the_flag() {
        let flag = CancelFlag::new();
        let clone = flag.clone();
        assert!(!clone.is_cancelled(), "a new flag is not cancelled");
        flag.cancel();
        assert!(clone.is_cancelled(), "cancelling one clone cancels all");
    }
}
