//! Error returned by `CorpusSource` adapters.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CorpusSourceError {
    #[error("corpus source '{adapter}' is unavailable: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },
}
