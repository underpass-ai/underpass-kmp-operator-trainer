use operator_shared_domain::contract::contract_violations::ContractViolations;
use thiserror::Error;

/// Error returned by [`crate::use_cases::validate_trajectory_use_case::ValidateTrajectoryUseCase`].
///
/// The use case only forwards contract violations from the injected
/// validator; it never produces an `ApplicationError`. Other use cases
/// that need broader error variants compose their own enum.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidateTrajectoryError {
    #[error("trajectory failed the action contract with {violation_count} violation(s)")]
    ContractViolations {
        violations: ContractViolations,
        violation_count: usize,
    },
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
