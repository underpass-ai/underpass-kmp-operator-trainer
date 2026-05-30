use thiserror::Error;

/// Why a window-expansion generation run failed for a specific episode. Each
/// variant carries the episode index so a failing row can be located in the
/// source. Messages are stringified from the underlying compile/session/expand
/// errors to keep this type cheap to clone and compare.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenerateWindowExpansionsError {
    #[error("episode {index}: failed to build session id: {message}")]
    SessionId { index: usize, message: String },

    #[error("episode {index}: failed to compile request: {message}")]
    Compile { index: usize, message: String },

    #[error("episode {index}: session run failed: {message}")]
    Session { index: usize, message: String },

    #[error("episode {index}: trajectory expansion failed: {message}")]
    Expand { index: usize, message: String },
}
