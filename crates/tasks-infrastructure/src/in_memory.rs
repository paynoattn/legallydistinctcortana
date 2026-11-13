use std::sync::{PoisonError, RwLock};

use tasks_application::ports::{RepositoryError, TaskRepository};
use tasks_domain::{Task, TaskId};

/// Keeps tasks in a `Vec` behind a lock, so insertion order is creation order.
///
/// A poisoned lock is recovered rather than reported: every write is a single
/// `push` or assignment, so a panic elsewhere cannot leave the list half-updated.
#[derive(Debug, Default)]
pub struct InMemoryTaskRepository {
    tasks: RwLock<Vec<Task>>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn save(&self, task: Task) -> Result<(), RepositoryError> {
        let mut tasks = self.tasks.write().unwrap_or_else(PoisonError::into_inner);
        match tasks.iter_mut().find(|stored| stored.id() == task.id()) {
            Some(stored) => *stored = task,
            None => tasks.push(task),
        }
        Ok(())
    }

    fn find(&self, id: &TaskId) -> Result<Option<Task>, RepositoryError> {
        let tasks = self.tasks.read().unwrap_or_else(PoisonError::into_inner);
        Ok(tasks.iter().find(|task| task.id() == id).cloned())
    }

    fn list(&self) -> Result<Vec<Task>, RepositoryError> {
        Ok(self
            .tasks
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tasks_domain::TaskTitle;

    use super::*;

    fn task(id: &str, title: &str) -> Task {
        Task::new(TaskId::new(id), TaskTitle::parse(title).unwrap())
    }

    #[test]
    fn starts_empty() {
        let repository = InMemoryTaskRepository::default();
        assert!(repository.list().unwrap().is_empty(), "no tasks yet");
        assert_eq!(repository.find(&TaskId::new("a")).unwrap(), None);
    }

    #[test]
    fn finds_saved_tasks_by_id() {
        let repository = InMemoryTaskRepository::default();
        repository.save(task("a", "first")).unwrap();
        repository.save(task("b", "second")).unwrap();
        assert_eq!(
            repository.find(&TaskId::new("b")).unwrap(),
            Some(task("b", "second"))
        );
    }

    #[test]
    fn lists_in_insertion_order() {
        let repository = InMemoryTaskRepository::default();
        repository.save(task("b", "first")).unwrap();
        repository.save(task("a", "second")).unwrap();
        assert_eq!(
            repository.list().unwrap(),
            vec![task("b", "first"), task("a", "second")]
        );
    }

    #[test]
    fn saving_an_existing_id_replaces_it_in_place() {
        let repository = InMemoryTaskRepository::default();
        repository.save(task("a", "first")).unwrap();
        repository.save(task("b", "second")).unwrap();
        let mut done = task("a", "first");
        done.complete();
        repository.save(done.clone()).unwrap();
        assert_eq!(repository.list().unwrap(), vec![done, task("b", "second")]);
    }

    #[test]
    fn keeps_working_after_a_panic_while_locked() {
        let repository = Arc::new(InMemoryTaskRepository::default());
        let poisoner = Arc::clone(&repository);
        let result = std::thread::spawn(move || {
            let _guard = poisoner.tasks.write().unwrap();
            panic!("poison the lock");
        })
        .join();
        assert!(result.is_err(), "thread panicked");
        assert!(repository.tasks.is_poisoned(), "lock is poisoned");

        repository.save(task("a", "after")).unwrap();
        assert_eq!(
            repository.find(&TaskId::new("a")).unwrap(),
            Some(task("a", "after"))
        );
        assert_eq!(repository.list().unwrap(), vec![task("a", "after")]);
    }
}
