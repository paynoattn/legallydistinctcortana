//! HTTP interface for the task use cases. Routes and JSON mapping only; no business rules.

mod dto;
mod error;
mod routes;

pub use routes::router;
