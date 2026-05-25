use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OperatorPolicyError {
    #[error("operator policy transport error: {message}")]
    Transport { message: String },

    #[error("operator policy protocol error: {message}")]
    Protocol { message: String },

    #[error("operator policy returned invalid action shape: {message}")]
    Shape { message: String },
}
