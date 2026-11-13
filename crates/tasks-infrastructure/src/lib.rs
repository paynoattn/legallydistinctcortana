//! Adapters that implement the `tasks-application` ports.

mod in_memory;
mod uuid_ids;

pub use in_memory::InMemoryTaskRepository;
pub use uuid_ids::UuidGenerator;
