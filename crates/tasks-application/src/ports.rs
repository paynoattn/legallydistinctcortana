//! Interfaces the use cases depend on. Adapters in `tasks-infrastructure` implement them,
//! and the composition root (`app`) injects the adapters.

use tasks_domain::{Task, TaskId};
use thiserror::Error;

/// Storage failed. Adapters put the cause in the message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("task storage failed: {0}")]
pub struct RepositoryError(pub String);

/// Stores tasks and returns them in creation order.
pub trait TaskRepository: Send + Sync {
    /// Inserts `task`, or replaces the stored task with the same id in place.
    ///
    /// # Errors
    ///
    /// [`RepositoryError`] if the store cannot be written.
    fn save(&self, task: Task) -> Result<(), RepositoryError>;

    /// The task with `id`, if any.
    ///
    /// # Errors
    ///
    /// [`RepositoryError`] if the store cannot be read.
    fn find(&self, id: &TaskId) -> Result<Option<Task>, RepositoryError>;

    /// All tasks in creation order.
    ///
    /// # Errors
    ///
    /// [`RepositoryError`] if the store cannot be read.
    fn list(&self) -> Result<Vec<Task>, RepositoryError>;
}

/// Produces a fresh, unique [`TaskId`] on every call.
pub trait IdGenerator: Send + Sync {
    /// A new id that no existing task has.
    fn next_id(&self) -> TaskId;
}
