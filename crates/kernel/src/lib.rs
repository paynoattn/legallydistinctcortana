//! Shared kernel: value types that more than one context needs. No dependencies, no I/O.

mod mood;

pub use mood::{Mood, UnknownMood};
