//! Error returned when realistic corpus event streaming cannot be persisted.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CorpusEventSinkError {
    #[error("{adapter} corpus event sink error: {message}")]
    Sink {
        adapter: &'static str,
        message: String,
    },
}
