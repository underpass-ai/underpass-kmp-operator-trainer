//! Error returned by the `Predictor` port.
//!
//! Note on shape vs. `TrainerInvocationOutcome`: the trainer port
//! models success and failure as **typed value variants** of the
//! outcome (`Success`/`Failed`) because both paths are meaningful to
//! callers — even a failed training run produces an artifact you can
//! inspect. The predictor port models failure as an **error type**
//! because the downstream validation flow has nothing to score when
//! the predictor cannot run to completion: there is no `Success`/
//! `Failed` value because the only success case is "ran cleanly and
//! left predictions on disk". Different roles in the flow, different
//! shapes.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PredictorError {
    /// The adapter could not start the predictor process — command
    /// not found, permission denied, working directory missing.
    #[error("predictor '{adapter}' could not start '{command}': {message}")]
    SpawnFailure {
        adapter: &'static str,
        command: String,
        message: String,
    },

    /// The adapter could not prepare or open the output directory
    /// (parent missing, permission denied, path is a file, …) before
    /// invoking the child. Distinct from `SpawnFailure` so callers
    /// can fix filesystem issues without thinking the binary is
    /// broken.
    #[error("predictor '{adapter}' output directory '{output_directory}' unusable: {message}")]
    OutputDirectoryUnusable {
        adapter: &'static str,
        output_directory: String,
        message: String,
    },

    /// The predictor process ran but exited non-zero. `exit_code` is
    /// `None` for processes killed by a signal.
    #[error("predictor '{adapter}' exited non-zero for '{command}': {exit_code:?}")]
    NonZeroExit {
        adapter: &'static str,
        command: String,
        exit_code: Option<i32>,
    },

    /// The predictor process exited cleanly but its
    /// `summary.json` is missing or unparseable. Distinct from
    /// `SpawnFailure` and `NonZeroExit` so callers can tell "the
    /// binary did not produce the expected output" apart from "the
    /// binary failed to start" or "the binary returned non-zero".
    #[error("predictor '{adapter}' summary at '{summary_path}' unreadable: {message}")]
    SummaryUnreadable {
        adapter: &'static str,
        summary_path: String,
        message: String,
    },
}
