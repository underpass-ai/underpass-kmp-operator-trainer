//! Error returned by the `PolicyEvaluator` port. Adapter-agnostic
//! shape; the adapter wraps domain errors from
//! `EvaluateOperatorPolicyUseCase` (or whatever its concrete evaluator
//! is) into one of two categories so the use case can tell adapter
//! failures apart from genuine domain inconsistencies.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyEvaluatorError {
    /// The evaluator received pairs whose ground-truth and prediction
    /// trajectory ids disagree, or some other domain-level
    /// inconsistency surfaced by the wrapped evaluator. Use this when
    /// the failure is a contract violation visible at the domain
    /// layer.
    #[error("policy evaluator '{adapter}' domain failure: {message}")]
    DomainFailure {
        adapter: &'static str,
        message: String,
    },

    /// The evaluator's underlying adapter failed for a reason that is
    /// not a domain inconsistency — e.g., transient I/O, a wrapped
    /// network call, a panic recovered by the adapter, or any failure
    /// that does not reflect "your inputs are wrong" but rather "the
    /// adapter could not complete the work".
    #[error("policy evaluator '{adapter}' adapter failure: {message}")]
    AdapterFailure {
        adapter: &'static str,
        message: String,
    },
}
