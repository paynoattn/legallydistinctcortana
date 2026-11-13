//! Runs `features/core/*.feature` against the composed application, in process.
#![expect(
    clippy::needless_pass_by_value,
    reason = "cucumber step captures must be owned FromStr types; its macros reject &str"
)]

use std::fmt::Debug;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use cucumber::{World, given, then, when};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

type StepResult = Result<(), String>;

#[derive(Debug, World)]
#[world(init = Self::new)]
struct TasksWorld {
    app: Router,
    status: StatusCode,
    body: Value,
    task_id: Option<String>,
}

impl TasksWorld {
    fn new() -> Self {
        Self {
            app: app::router(),
            status: StatusCode::IM_A_TEAPOT,
            body: Value::Null,
            task_id: None,
        }
    }

    async fn send(&mut self, request: Result<Request<Body>, axum::http::Error>) -> StepResult {
        let request = request.map_err(|e| e.to_string())?;
        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .map_err(|e| e.to_string())?;
        self.status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| e.to_string())?
            .to_bytes();
        self.body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(())
    }

    async fn send_empty(&mut self, method: &str, path: &str) -> StepResult {
        self.send(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty()),
        )
        .await
    }

    async fn post_json(&mut self, path: &str, body: String) -> StepResult {
        let request = Request::post(path).header(header::CONTENT_TYPE, "application/json");
        self.send(request.body(Body::from(body))).await
    }

    async fn create(&mut self, title: &str) -> StepResult {
        self.post_json("/tasks", json!({ "title": title }).to_string())
            .await
    }

    fn task_path(&self, suffix: &str) -> Result<String, String> {
        let id = self
            .task_id
            .as_ref()
            .ok_or("no task was created in this scenario")?;
        Ok(format!("/tasks/{id}{suffix}"))
    }

    fn field(&self, name: &str) -> &Value {
        self.body.get(name).unwrap_or(&Value::Null)
    }
}

fn expect_eq<T: PartialEq + Debug>(actual: &T, expected: &T, what: &str) -> StepResult {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: expected {expected:?}, got {actual:?}"))
    }
}

#[given(expr = "a task titled {string} exists")]
async fn task_exists(world: &mut TasksWorld, title: String) -> StepResult {
    world.create(&title).await?;
    expect_eq(&world.status, &StatusCode::CREATED, "create status")?;
    let id = world
        .field("id")
        .as_str()
        .ok_or("created task has no string id")?;
    world.task_id = Some(id.to_owned());
    Ok(())
}

#[when(regex = r"^I send (GET|POST) (\S+)$")]
async fn send_request(world: &mut TasksWorld, method: String, path: String) -> StepResult {
    world.send_empty(&method, &path).await
}

#[when(regex = r"^I send POST (\S+) with body (.+)$")]
async fn send_post_with_body(world: &mut TasksWorld, path: String, body: String) -> StepResult {
    world.post_json(&path, body).await
}

#[when(expr = "I create a task titled {string}")]
async fn create_task(world: &mut TasksWorld, title: String) -> StepResult {
    world.create(&title).await
}

#[when(regex = r"^I create a task with a title of (\d+) (letter|space) characters$")]
async fn create_task_of_length(world: &mut TasksWorld, length: usize, kind: String) -> StepResult {
    let unit = if kind == "space" { " " } else { "a" };
    world.create(&unit.repeat(length)).await
}

#[when("I fetch that task")]
async fn fetch_task(world: &mut TasksWorld) -> StepResult {
    let path = world.task_path("")?;
    world.send_empty("GET", &path).await
}

#[when("I complete that task")]
async fn complete_task(world: &mut TasksWorld) -> StepResult {
    let path = world.task_path("/complete")?;
    world.send_empty("POST", &path).await
}

#[then(expr = "the response status is {int}")]
fn response_status(world: &mut TasksWorld, status: u16) -> StepResult {
    expect_eq(&world.status.as_u16(), &status, "status")
}

#[then(regex = r"^the response body is (.+)$")]
fn response_body(world: &mut TasksWorld, expected: Value) -> StepResult {
    expect_eq(&world.body, &expected, "body")
}

#[then(regex = r#"^the response is an? (open|done) task titled "(.*)"$"#)]
fn response_task(world: &mut TasksWorld, state: String, title: String) -> StepResult {
    expect_eq(world.field("title"), &json!(title), "title")?;
    expect_eq(world.field("done"), &json!(state == "done"), "done")?;
    let has_id = world.field("id").as_str().is_some_and(|id| !id.is_empty());
    expect_eq(&has_id, &true, "has a non-empty id")
}

#[then(expr = "the task titles are {string}")]
fn task_titles(world: &mut TasksWorld, titles: String) -> StepResult {
    let tasks = world.body.as_array().ok_or("body is not a list")?;
    let actual: Vec<&str> = tasks
        .iter()
        .filter_map(|task| task.get("title")?.as_str())
        .collect();
    let expected: Vec<&str> = titles.split(", ").collect();
    expect_eq(&actual, &expected, "titles")
}

#[tokio::main]
async fn main() {
    TasksWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../../features/core"))
        .await;
}
