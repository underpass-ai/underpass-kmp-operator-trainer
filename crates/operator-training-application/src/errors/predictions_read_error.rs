//! Error returned by the predictions reader port. Adapter-agnostic
//! variants so filesystem, network and stream readers all map onto
//! the same shape. The infra crate's own
//! `PredictionsReadError` is converted into this shape by the adapter
//! before it crosses the use-case boundary.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PredictionsReadError {
    #[error("predictions reader '{adapter}' unavailable: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },

    #[error("predictions reader '{adapter}' invalid row at line {line}: {message}")]
    InvalidRow {
        adapter: &'static str,
        line: usize,
        message: String,
    },

    #[error("predictions reader '{adapter}' shape violation at line {line}: {message}")]
    ShapeViolation {
        adapter: &'static str,
        line: usize,
        message: String,
    },
}
