//! Error returned by the `Predictor` port.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PredictorError {
    /// The adapter could not run the predictor process — command not
    /// found, permission denied, working directory missing, or the
    /// parent could not collect the child's exit status.
    #[error("predictor '{adapter}' could not run '{command}': {message}")]
    SpawnFailure {
        adapter: &'static str,
        command: String,
        message: String,
    },

    /// The predictor process ran but exited with a non-zero status.
    /// The exit code is `None` for processes killed by a signal.
    #[error("predictor '{adapter}' exited non-zero for '{command}': {exit_code:?}")]
    NonZeroExit {
        adapter: &'static str,
        command: String,
        exit_code: Option<i32>,
    },
}
