use crate::TaskTitle;

/// Opaque task identifier. The format is chosen by the `IdGenerator` adapter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// Wraps an identifier produced by an id generator or received from a client.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A to-do item. Starts open; completing it is idempotent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    id: TaskId,
    title: TaskTitle,
    done: bool,
}

impl Task {
    /// A new, open task.
    #[must_use]
    pub fn new(id: TaskId, title: TaskTitle) -> Self {
        Self {
            id,
            title,
            done: false,
        }
    }

    /// Marks the task done. Completing a done task leaves it done.
    pub fn complete(&mut self) {
        self.done = true;
    }

    /// The task's identifier.
    #[must_use]
    pub fn id(&self) -> &TaskId {
        &self.id
    }

    /// The task's title.
    #[must_use]
    pub fn title(&self) -> &TaskTitle {
        &self.title
    }

    /// Whether the task has been completed.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_task() -> Task {
        Task::new(TaskId::new("t-1"), TaskTitle::parse("write tests").unwrap())
    }

    #[test]
    fn new_task_is_open_and_keeps_its_fields() {
        let task = open_task();
        assert_eq!(task.id().as_str(), "t-1");
        assert_eq!(task.title().as_str(), "write tests");
        assert!(!task.is_done(), "new tasks start open");
    }

    #[test]
    fn complete_marks_done() {
        let mut task = open_task();
        task.complete();
        assert!(task.is_done(), "completed task is done");
    }

    #[test]
    fn complete_is_idempotent() {
        let mut task = open_task();
        task.complete();
        task.complete();
        assert!(task.is_done(), "completing twice keeps it done");
    }
}
