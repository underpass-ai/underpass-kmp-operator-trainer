use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WindowExpansionEpisodeSourceError {
    #[error("{adapter} source unavailable: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },

    #[error("{adapter} invalid row {line}: {message}")]
    InvalidRow {
        adapter: &'static str,
        line: usize,
        message: String,
    },

    #[error("{adapter} source is empty")]
    EmptySource { adapter: &'static str },
}
