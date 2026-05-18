//! Domain error variants for the replay bounded context.

use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReplayDomainError {
    #[error("replay report must contain at least one execution")]
    EmptyReport,

    #[error(transparent)]
    Shared(#[from] DomainError),
}
