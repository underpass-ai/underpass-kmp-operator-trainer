//! Error returned by the `PolicyEvaluator` port. Adapter-agnostic
//! shape; the adapter wraps domain errors from
//! `EvaluateOperatorPolicyUseCase` into this variant set.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyEvaluatorError {
    /// The evaluator received pairs whose ground-truth and prediction
    /// trajectory ids disagree, or some other domain-level
    /// inconsistency.
    #[error("policy evaluator '{adapter}' domain failure: {message}")]
    DomainFailure {
        adapter: &'static str,
        message: String,
    },
}
