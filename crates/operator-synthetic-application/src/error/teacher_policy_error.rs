//! Error returned by a teacher policy adapter.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum TeacherPolicyError {
    #[error("{adapter} transport error: {message}")]
    Transport {
        adapter: &'static str,
        message: String,
    },

    #[error("{adapter} API error {code:?}: {message}")]
    ApiError {
        adapter: &'static str,
        code: Option<String>,
        message: String,
    },

    #[error("{adapter} protocol error: {message}")]
    Protocol {
        adapter: &'static str,
        message: String,
    },

    #[error("{adapter} produced invalid action shape: {message}")]
    Shape {
        adapter: &'static str,
        message: String,
    },
}
