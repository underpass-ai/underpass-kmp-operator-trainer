//! Error raised while mapping scenario DTOs to application values.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_infra::mappers::mapping_error::MappingError;
use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use thiserror::Error;

use crate::errors::calibration_case_mapping_error::CalibrationCaseMappingError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScenarioMappingError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Shared(#[from] MappingError),

    #[error(transparent)]
    Synthetic(#[from] SyntheticDomainError),

    #[error(transparent)]
    Calibration(#[from] CalibrationCaseMappingError),

    #[error("scenario subject serialization failed: {message}")]
    Serialization { message: String },
}
