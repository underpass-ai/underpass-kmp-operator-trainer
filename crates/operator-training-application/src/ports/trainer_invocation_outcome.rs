//! Typed value object returned by `TrainerInvoker::invoke`. Records
//! whether the external trainer exited cleanly and, if available,
//! the numeric exit code. Signal-killed processes have no exit code
//! and are represented as `Failed` with `exit_code: None`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerInvocationOutcome {
    /// The trainer exited with status zero.
    Success { exit_code: i32 },
    /// The trainer exited with a non-zero status (or was killed by a
    /// signal, in which case `exit_code` is `None`).
    Failed { exit_code: Option<i32> },
}

impl TrainerInvocationOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}
