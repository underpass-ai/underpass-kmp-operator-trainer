//! Error returned while loading teacher calibration cases.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CalibrationEpisodeSourceError {
    #[error("{adapter} could not read calibration source: {message}")]
    SourceUnavailable {
        adapter: &'static str,
        message: String,
    },

    #[error("{adapter} calibration source is empty")]
    EmptySource { adapter: &'static str },

    #[error("{adapter} invalid row {line}: {message}")]
    InvalidRow {
        adapter: &'static str,
        line: usize,
        message: String,
    },
}
