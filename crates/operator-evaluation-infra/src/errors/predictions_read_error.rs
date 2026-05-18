//! Error returned by `JsonlPredictionsReader`. Adapter-agnostic shape
//! so a future stream adapter (NATS, Kafka) can reuse the same
//! variants.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PredictionsReadError {
    /// The adapter could not open or read bytes from the source —
    /// file missing, permission denied, broken pipe.
    #[error("predictions reader '{adapter}' could not read source: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },

    /// A row was syntactically invalid (bad JSON, wrong root type).
    /// The line number is 1-based.
    #[error("predictions reader '{adapter}' invalid row at line {line}: {message}")]
    InvalidRow {
        adapter: &'static str,
        line: usize,
        message: String,
    },

    /// A row parsed but its shape did not match the predictions
    /// contract — missing `step_id`, missing `action`, or the action
    /// itself failed domain mapping.
    #[error("predictions reader '{adapter}' shape violation at line {line}: {message}")]
    ShapeViolation {
        adapter: &'static str,
        line: usize,
        message: String,
    },
}
