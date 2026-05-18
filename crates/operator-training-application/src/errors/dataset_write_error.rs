//! Error returned by the `DatasetWriter` port.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DatasetWriteError {
    /// The adapter failed to write bytes — permission denied, disk
    /// full, broken pipe, etc.
    #[error("dataset writer '{adapter}' write failed: {message}")]
    WriteFailure {
        adapter: &'static str,
        message: String,
    },

    /// The adapter wrote bytes but content hashing failed.
    #[error("dataset writer '{adapter}' could not hash content: {message}")]
    HashingFailure {
        adapter: &'static str,
        message: String,
    },

    /// The adapter could not compute a derived value object from the
    /// written content (e.g., the trajectory count overflows a
    /// `PositiveCount`). Use this when the failure is shape-level,
    /// not transport-level.
    #[error("dataset writer '{adapter}' derived value error: {message}")]
    DerivedValueFailure {
        adapter: &'static str,
        message: String,
    },
}
