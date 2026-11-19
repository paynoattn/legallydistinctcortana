//! Helpers the wiring shares: named background threads, and failures that are logged
//! because there is no one to return them to.

use std::fmt::Display;
use std::thread;

/// Runs `task` on a new thread named `name`, logging if the thread can't start.
pub(crate) fn spawn(name: &str, task: impl FnOnce() + Send + 'static) {
    let spawned = thread::Builder::new().name(name.into()).spawn(task);
    log_failure(spawned.map(drop), "start a thread");
}

/// Logs `result` as "could not {action}" if it failed.
pub(crate) fn log_failure<E: Display>(result: Result<(), E>, action: &str) {
    if let Err(error) = result {
        tracing::warn!(%error, "could not {action}");
    }
}
