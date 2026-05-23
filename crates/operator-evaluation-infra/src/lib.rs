//! Operator evaluation bounded context — infrastructure adapters.
//!
//! Reads wire artifacts produced by external evaluators (today: the
//! Python `predict_operator_sft.py` script that ships in
//! `scripts/operator/`) and maps them onto typed domain values.
//!
//! - `JsonlPredictionsReader` — parses `predictions.jsonl`, the
//!   per-step `{step_id, action}` shape that the Python predictor
//!   emits, into a `PredictionsReadOutcome`. Callers join parsed
//!   step-keyed predictions with their ground-truth trajectories and
//!   pass recoverable shape violations through to the report.
//!
//! See [ADR 0012](../../../docs/architecture/operator/decisions/0012-training-context-design.md)
//! for the broader training/validation pipeline.

pub mod adapters;
pub mod dto;
pub mod errors;
