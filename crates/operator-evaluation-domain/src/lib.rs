//! Operator: evaluation bounded context — domain.
//!
//! Owns the canonical types for scoring an Operator policy against a
//! ground-truth dataset: `PredictedAction`, `EvaluationPair`,
//! `PredictionEvaluationOutcome`, `ToolEvaluationMetric`,
//! `EvaluationReport` and `EvaluationDomainError`.

pub mod error;
pub mod outcome;
pub mod prediction;
pub mod report;
