//! Error returned by a teacher policy adapter.

use operator_shared_domain::value_objects::finish_reason::FinishReason;
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
        finish_reason: Option<FinishReason>,
    },

    #[error("{adapter} returned non-stop finish reason {finish_reason}: content_len={content_len}")]
    TruncatedResponse {
        adapter: &'static str,
        finish_reason: FinishReason,
        content_len: usize,
    },
}
