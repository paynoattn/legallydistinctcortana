use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tasks_application::TaskError;

use crate::dto::ErrorResponse;

/// Every failure a handler can return, mapped to a status code and `{"error": "..."}`.
#[derive(Debug)]
pub(crate) enum ApiError {
    MalformedBody(JsonRejection),
    Task(TaskError),
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::MalformedBody(rejection)
    }
}

impl From<TaskError> for ApiError {
    fn from(error: TaskError) -> Self {
        Self::Task(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MalformedBody(rejection) => (StatusCode::BAD_REQUEST, rejection.body_text()),
            Self::Task(error) => task_failure(&error),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

fn task_failure(error: &TaskError) -> (StatusCode, String) {
    match error {
        TaskError::InvalidTitle(_) => (StatusCode::BAD_REQUEST, error.to_string()),
        TaskError::NotFound(_) => (StatusCode::NOT_FOUND, error.to_string()),
        TaskError::Repository(cause) => {
            tracing::error!(%cause, "repository failure");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_owned(),
            )
        }
    }
}
