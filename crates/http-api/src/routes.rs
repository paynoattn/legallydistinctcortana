use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tasks_application::TaskService;
use tasks_domain::TaskId;

use crate::dto::{CreateTaskRequest, TaskResponse};
use crate::error::ApiError;

/// All routes, with `service` as shared state.
pub fn router(service: TaskService) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", get(get_task))
        .route("/tasks/{id}/complete", post(complete_task))
        .with_state(service)
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn create_task(
    State(service): State<TaskService>,
    body: Result<Json<CreateTaskRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiError> {
    let Json(request) = body?;
    let task = service.create(&request.title)?;
    Ok((StatusCode::CREATED, Json(TaskResponse::from(&task))))
}

async fn list_tasks(
    State(service): State<TaskService>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let tasks = service.list()?;
    Ok(Json(tasks.iter().map(TaskResponse::from).collect()))
}

async fn get_task(
    State(service): State<TaskService>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, ApiError> {
    let task = service.get(&TaskId::new(id))?;
    Ok(Json(TaskResponse::from(&task)))
}

async fn complete_task(
    State(service): State<TaskService>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, ApiError> {
    let task = service.complete(&TaskId::new(id))?;
    Ok(Json(TaskResponse::from(&task)))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tasks_application::ports::{IdGenerator, RepositoryError, TaskRepository};
    use tasks_domain::Task;
    use tower::ServiceExt;

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
            Err(RepositoryError("secret detail".into()))
        }

        fn find(&self, _id: &TaskId) -> Result<Option<Task>, RepositoryError> {
            Err(RepositoryError("secret detail".into()))
        }

        fn list(&self) -> Result<Vec<Task>, RepositoryError> {
            Err(RepositoryError("secret detail".into()))
        }
    }

    #[derive(Default)]
    struct SequentialIds(AtomicUsize);

    impl IdGenerator for SequentialIds {
        fn next_id(&self) -> TaskId {
            TaskId::new(format!("id-{}", self.0.fetch_add(1, Ordering::SeqCst)))
        }
    }

    fn app() -> Router {
        router(TaskService::new(
            Arc::new(VecRepository::default()),
            Arc::new(SequentialIds::default()),
        ))
    }

    fn broken_app() -> Router {
        router(TaskService::new(
            Arc::new(BrokenRepository),
            Arc::new(SequentialIds::default()),
        ))
    }

    async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
        send(app, Request::get(uri).body(Body::empty()).unwrap()).await
    }

    async fn post(app: &Router, uri: &str, json_body: &str) -> (StatusCode, Value) {
        let request = Request::post(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_body.to_owned()))
            .unwrap();
        send(app, request).await
    }

    async fn post_empty(app: &Router, uri: &str) -> (StatusCode, Value) {
        send(app, Request::post(uri).body(Body::empty()).unwrap()).await
    }

    async fn create(app: &Router, title: &str) -> Value {
        post(app, "/tasks", &json!({ "title": title }).to_string())
            .await
            .1
    }

    #[tokio::test]
    async fn health_is_ok() {
        assert_eq!(
            get(&app(), "/health").await,
            (StatusCode::OK, json!({"status": "ok"}))
        );
    }

    #[tokio::test]
    async fn create_returns_201_with_an_open_trimmed_task() {
        let response = post(&app(), "/tasks", r#"{"title":"  buy milk "}"#).await;
        assert_eq!(
            response,
            (
                StatusCode::CREATED,
                json!({"id": "id-0", "title": "buy milk", "done": false})
            )
        );
    }

    #[tokio::test]
    async fn create_with_invalid_title_is_400_with_reason() {
        let response = post(&app(), "/tasks", r#"{"title":"   "}"#).await;
        assert_eq!(
            response,
            (
                StatusCode::BAD_REQUEST,
                json!({"error": "title must not be empty"})
            )
        );
    }

    #[tokio::test]
    async fn create_with_malformed_body_is_400() {
        for body in [r#"{"name":"x"}"#, "not json", r#"{"title":5}"#] {
            let (status, response) = post(&app(), "/tasks", body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "body {body}");
            assert!(response["error"].is_string(), "error message for {body}");
        }
    }

    #[tokio::test]
    async fn list_returns_tasks_in_creation_order() {
        let app = app();
        let first = create(&app, "first").await;
        let second = create(&app, "second").await;
        assert_eq!(
            get(&app, "/tasks").await,
            (StatusCode::OK, json!([first, second]))
        );
    }

    #[tokio::test]
    async fn get_returns_the_task_or_404() {
        let app = app();
        let task = create(&app, "find me").await;
        assert_eq!(get(&app, "/tasks/id-0").await, (StatusCode::OK, task));
        assert_eq!(
            get(&app, "/tasks/missing").await,
            (
                StatusCode::NOT_FOUND,
                json!({"error": "task missing not found"})
            )
        );
    }

    #[tokio::test]
    async fn complete_marks_done_idempotently_or_404s() {
        let app = app();
        create(&app, "finish").await;
        let done = (
            StatusCode::OK,
            json!({"id": "id-0", "title": "finish", "done": true}),
        );
        assert_eq!(post_empty(&app, "/tasks/id-0/complete").await, done);
        assert_eq!(post_empty(&app, "/tasks/id-0/complete").await, done);
        assert_eq!(
            post_empty(&app, "/tasks/missing/complete").await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn repository_failure_is_500_without_leaking_details() {
        let app = broken_app();
        let expected = (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "internal server error"}),
        );
        assert_eq!(get(&app, "/tasks").await, expected);
        assert_eq!(get(&app, "/tasks/x").await, expected);
        assert_eq!(post_empty(&app, "/tasks/x/complete").await, expected);
        assert_eq!(post(&app, "/tasks", r#"{"title":"t"}"#).await, expected);
    }
}
