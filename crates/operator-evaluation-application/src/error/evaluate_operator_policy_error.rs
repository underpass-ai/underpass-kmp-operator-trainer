use operator_evaluation_domain::error::evaluation_domain_error::EvaluationDomainError;
use thiserror::Error;

/// Error returned by `EvaluateOperatorPolicyUseCase`. Today the use case
/// only forwards evaluation-domain errors raised during outcome
/// assembly; the variant exists so future use cases can compose
/// additional sources without breaking the public type.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluateOperatorPolicyError {
    #[error(transparent)]
    Domain(#[from] EvaluationDomainError),
}
