//! Domain error variants for the synthetic bounded context.

use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyntheticDomainError {
    #[error("synthetic blueprint must contain at least one case")]
    EmptyBlueprint,

    #[error("synthetic case '{case_id}' has duplicate occurrences in the blueprint")]
    DuplicateCase { case_id: String },

    #[error(transparent)]
    Shared(#[from] DomainError),
}
