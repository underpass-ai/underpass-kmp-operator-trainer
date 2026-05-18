use operator_shared_domain::contract::contract_violations::ContractViolations;
use thiserror::Error;

use crate::error::application_error::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidateTrajectoryError {
    #[error("trajectory failed the action contract with {violation_count} violation(s)")]
    ContractViolations {
        violations: ContractViolations,
        violation_count: usize,
    },

    #[error(transparent)]
    Application(#[from] ApplicationError),
}

impl ValidateTrajectoryError {
    pub fn from_violations(violations: ContractViolations) -> Self {
        let violation_count = violations.len();
        Self::ContractViolations {
            violations,
            violation_count,
        }
    }
}
