//! Error returned by the `TrainerInvoker` port.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrainerInvokerError {
    /// The adapter could not run the trainer process to completion —
    /// command not found, permission denied, working directory
    /// missing, or the parent could not collect the child's exit
    /// status. Spawn and wait are surfaced as a single failure mode
    /// because every shipped adapter uses `Command::status()` or
    /// equivalents that combine them; a future adapter that splits
    /// spawn from wait can introduce a dedicated variant in the
    /// same PR that needs it.
    #[error("trainer invoker '{adapter}' could not run '{command}': {message}")]
    SpawnFailure {
        adapter: &'static str,
        command: String,
        message: String,
    },
}
