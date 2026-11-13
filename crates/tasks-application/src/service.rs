use std::fmt;
use std::sync::Arc;

use tasks_domain::{Task, TaskId, TaskTitle, TitleError};
use thiserror::Error;

use crate::ports::{IdGenerator, RepositoryError, TaskRepository};

/// Why a task use case failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TaskError {
    /// The requested title is not valid.
    #[error(transparent)]
    InvalidTitle(#[from] TitleError),
    /// No task has the given id.
    #[error("task {} not found", .0.as_str())]
    NotFound(TaskId),
    /// The repository failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// The task use cases: create, list, get, complete.
#[derive(Clone)]
pub struct TaskService {
    repository: Arc<dyn TaskRepository>,
    ids: Arc<dyn IdGenerator>,
}

impl fmt::Debug for TaskService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskService").finish_non_exhaustive()
    }
}

impl TaskService {
    /// A service backed by the given adapters.
    #[must_use]
    pub fn new(repository: Arc<dyn TaskRepository>, ids: Arc<dyn IdGenerator>) -> Self {
        Self { repository, ids }
    }

    /// Validates `title` and stores a new open task with it.
    ///
    /// # Errors
    ///
    /// [`TaskError::InvalidTitle`] or [`TaskError::Repository`].
    pub fn create(&self, title: &str) -> Result<Task, TaskError> {
        let title = TaskTitle::parse(title)?;
        let task = Task::new(self.ids.next_id(), title);
        self.repository.save(task.clone())?;
        Ok(task)
    }

    /// All tasks in creation order.
    ///
    /// # Errors
    ///
    /// [`TaskError::Repository`].
    pub fn list(&self) -> Result<Vec<Task>, TaskError> {
        Ok(self.repository.list()?)
    }

    /// The task with `id`.
    ///
    /// # Errors
    ///
    /// [`TaskError::NotFound`] or [`TaskError::Repository`].
    pub fn get(&self, id: &TaskId) -> Result<Task, TaskError> {
        self.repository
            .find(id)?
            .ok_or_else(|| TaskError::NotFound(id.clone()))
    }

    /// Marks the task with `id` done. Completing a done task succeeds unchanged.
    ///
    /// # Errors
    ///
    /// [`TaskError::NotFound`] or [`TaskError::Repository`].
    pub fn complete(&self, id: &TaskId) -> Result<Task, TaskError> {
        let mut task = self.get(id)?;
        task.complete();
        self.repository.save(task.clone())?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(Default)]
    struct VecRepository(Mutex<Vec<Task>>);

    impl TaskRepository for VecRepository {
        fn save(&self, task: Task) -> Result<(), RepositoryError> {
            let mut tasks = self.0.lock().unwrap();
            tasks.retain(|t| t.id() != task.id());
            tasks.push(task);
            Ok(())
        }

        fn find(&self, id: &TaskId) -> Result<Option<Task>, RepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|t| t.id() == id)
                .cloned())
        }

        fn list(&self) -> Result<Vec<Task>, RepositoryError> {
            Ok(self.0.lock().unwrap().clone())
        }
    }

    struct BrokenRepository;

    impl TaskRepository for BrokenRepository {
        fn save(&self, _task: Task) -> Result<(), RepositoryError> {
            Err(RepositoryError("disk full".into()))
        }

        fn find(&self, _id: &TaskId) -> Result<Option<Task>, RepositoryError> {
            Err(RepositoryError("disk full".into()))
        }

        fn list(&self) -> Result<Vec<Task>, RepositoryError> {
            Err(RepositoryError("disk full".into()))
        }
    }

    #[derive(Default)]
    struct SequentialIds(AtomicUsize);

    impl IdGenerator for SequentialIds {
        fn next_id(&self) -> TaskId {
            TaskId::new(format!("id-{}", self.0.fetch_add(1, Ordering::SeqCst)))
        }
    }

    fn service() -> TaskService {
        TaskService::new(
            Arc::new(VecRepository::default()),
            Arc::new(SequentialIds::default()),
        )
    }

    fn broken_service() -> TaskService {
        TaskService::new(
            Arc::new(BrokenRepository),
            Arc::new(SequentialIds::default()),
        )
    }

    #[test]
    fn create_stores_an_open_task_with_a_trimmed_title() {
        let service = service();
        let task = service.create("  buy milk ").unwrap();
        assert_eq!(task.id().as_str(), "id-0");
        assert_eq!(task.title().as_str(), "buy milk");
        assert!(!task.is_done(), "new tasks start open");
        assert_eq!(service.get(task.id()).unwrap(), task);
    }

    #[test]
    fn create_rejects_an_invalid_title_without_storing() {
        let service = service();
        assert_eq!(
            service.create("   "),
            Err(TaskError::InvalidTitle(TitleError::Empty))
        );
        assert!(service.list().unwrap().is_empty(), "nothing stored");
    }

    #[test]
    fn list_returns_tasks_in_creation_order() {
        let service = service();
        let first = service.create("first").unwrap();
        let second = service.create("second").unwrap();
        assert_eq!(service.list().unwrap(), vec![first, second]);
    }

    #[test]
    fn get_unknown_id_is_not_found() {
        let id = TaskId::new("nope");
        assert_eq!(service().get(&id), Err(TaskError::NotFound(id)));
    }

    #[test]
    fn complete_marks_the_stored_task_done() {
        let service = service();
        let task = service.create("finish").unwrap();
        let completed = service.complete(task.id()).unwrap();
        assert!(completed.is_done(), "returned task is done");
        assert!(
            service.get(task.id()).unwrap().is_done(),
            "stored task is done"
        );
    }

    #[test]
    fn complete_twice_is_idempotent() {
        let service = service();
        let task = service.create("finish").unwrap();
        let once = service.complete(task.id()).unwrap();
        let twice = service.complete(task.id()).unwrap();
        assert_eq!(once, twice);
        assert_eq!(service.list().unwrap().len(), 1);
    }

    #[test]
    fn complete_unknown_id_is_not_found() {
        let id = TaskId::new("nope");
        assert_eq!(service().complete(&id), Err(TaskError::NotFound(id)));
    }

    #[test]
    fn repository_failures_surface_as_repository_errors() {
        let service = broken_service();
        let failure = TaskError::Repository(RepositoryError("disk full".into()));
        let id = TaskId::new("x");
        assert_eq!(service.create("ok"), Err(failure.clone()));
        assert_eq!(service.list(), Err(failure.clone()));
        assert_eq!(service.get(&id), Err(failure.clone()));
        assert_eq!(service.complete(&id), Err(failure));
    }

    #[test]
    fn error_messages_are_readable() {
        assert_eq!(
            TaskError::NotFound(TaskId::new("abc")).to_string(),
            "task abc not found"
        );
        assert_eq!(
            TaskError::Repository(RepositoryError("boom".into())).to_string(),
            "task storage failed: boom"
        );
        assert_eq!(
            TaskError::from(TitleError::Empty).to_string(),
            "title must not be empty"
        );
    }

    #[test]
    fn debug_does_not_leak_adapters() {
        assert_eq!(format!("{:?}", service()), "TaskService { .. }");
    }
}
