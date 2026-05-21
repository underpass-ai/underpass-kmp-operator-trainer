//! Error returned by the episode splitter service.

use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EpisodeSplitError {
    #[error(transparent)]
    Domain(#[from] SyntheticDomainError),
}
