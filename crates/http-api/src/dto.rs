use serde::{Deserialize, Serialize};
use tasks_domain::Task;

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTaskRequest {
    pub(crate) title: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskResponse {
    id: String,
    title: String,
    done: bool,
}

impl From<&Task> for TaskResponse {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id().as_str().to_owned(),
            title: task.title().as_str().to_owned(),
            done: task.is_done(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}
