//! Error returned by the `TrainerInvoker` port.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrainerInvokerError {
    /// The adapter could not spawn the trainer process (command not
    /// found, permission denied, working directory missing).
    #[error("trainer invoker '{adapter}' spawn failed for '{command}': {message}")]
    SpawnFailure {
        adapter: &'static str,
        command: String,
        message: String,
    },

    /// The adapter spawned the process but waiting on it failed (the
    /// parent could not collect the exit status).
    #[error("trainer invoker '{adapter}' wait failed for '{command}': {message}")]
    WaitFailure {
        adapter: &'static str,
        command: String,
        message: String,
    },
}
