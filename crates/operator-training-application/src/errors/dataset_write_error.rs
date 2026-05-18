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

    /// The adapter could not compute a derived value object from the
    /// written content (e.g., the trajectory count overflows a
    /// `PositiveCount`, a `ContentHash` rejects the produced digest,
    /// or a `TaskFamilyDistribution` rejects a duplicate). Use this
    /// when the failure is shape-level, not transport-level.
    #[error("dataset writer '{adapter}' derived value error: {message}")]
    DerivedValueFailure {
        adapter: &'static str,
        message: String,
    },
}
