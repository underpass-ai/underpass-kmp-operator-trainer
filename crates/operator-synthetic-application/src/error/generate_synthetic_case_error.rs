//! Error returned by a `SyntheticCaseGenerator` adapter when generating
//! trajectories for one case. Carries the case identifier and a domain or
//! adapter-side reason; structured enough for callers to log without
//! exposing infrastructure detail.

use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenerateSyntheticCaseError {
    #[error(transparent)]
    Domain(#[from] SyntheticDomainError),

    #[error("synthetic case generator '{adapter}' failed for case '{case_id}': {message}")]
    Generator {
        adapter: &'static str,
        case_id: String,
        message: String,
    },
}
