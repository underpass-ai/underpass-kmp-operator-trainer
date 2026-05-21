//! Error raised while mapping calibration DTOs to domain objects.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_infra::mappers::mapping_error::MappingError;
use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CalibrationCaseMappingError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Shared(#[from] MappingError),

    #[error(transparent)]
    Synthetic(#[from] SyntheticDomainError),
}
