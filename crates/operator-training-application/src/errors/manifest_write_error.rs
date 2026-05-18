//! Error returned by the `ManifestWriter` port.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ManifestWriteError {
    #[error("manifest writer '{adapter}' write failed: {message}")]
    WriteFailure {
        adapter: &'static str,
        message: String,
    },

    /// The adapter could not serialise the manifest into its target
    /// format (e.g., TOML serialisation error).
    #[error("manifest writer '{adapter}' serialisation failed: {message}")]
    SerialisationFailure {
        adapter: &'static str,
        message: String,
    },
}
