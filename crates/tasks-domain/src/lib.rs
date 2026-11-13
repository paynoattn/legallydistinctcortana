//! Task entity and its value objects. Pure: no I/O, no framework, no async runtime.

mod task;
mod title;

pub use task::{Task, TaskId};
pub use title::{MAX_TITLE_CHARS, TaskTitle, TitleError};
