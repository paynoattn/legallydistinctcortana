//! Task use cases. Talks to the outside world only through the ports in [`ports`].

pub mod ports;
mod service;

pub use service::{TaskError, TaskService};
