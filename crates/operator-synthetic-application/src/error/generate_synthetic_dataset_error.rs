//! Error returned by `GenerateSyntheticDatasetUseCase`. Wraps the per-case
//! generator error and shared-context domain errors that bubble up when
//! assembling the final dataset.

use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use thiserror::Error;

use crate::error::generate_synthetic_case_error::GenerateSyntheticCaseError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenerateSyntheticDatasetError {
    #[error(transparent)]
    Case(#[from] GenerateSyntheticCaseError),

    #[error(transparent)]
    Domain(#[from] SyntheticDomainError),
}
