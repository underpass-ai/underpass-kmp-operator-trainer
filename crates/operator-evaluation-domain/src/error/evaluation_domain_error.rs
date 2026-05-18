//! Domain error variants for the evaluation bounded context.

use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluationDomainError {
    #[error(
        "predicted action's trajectory id '{actual}' does not match ground truth id '{expected}'"
    )]
    TrajectoryIdMismatch { expected: String, actual: String },

    #[error(transparent)]
    Shared(#[from] DomainError),
}
