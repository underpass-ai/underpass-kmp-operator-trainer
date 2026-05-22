//! Error returned while loading realistic corpus scenarios.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScenarioSourceError {
    #[error("{adapter} could not read scenario source: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },

    #[error("{adapter} scenario source is empty")]
    EmptySource { adapter: &'static str },

    #[error("{adapter} invalid row {line}: {message}")]
    InvalidRow {
        adapter: &'static str,
        line: usize,
        message: String,
    },
}
