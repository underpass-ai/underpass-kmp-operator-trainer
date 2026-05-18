//! Operator: replay bounded context — domain.
//!
//! Captures what happened when a predicted `OperatorAction` was
//! executed against a real KMP server: prediction, outcome (success /
//! no-call / failure), per-execution record, and aggregate report.

pub mod error;
pub mod execution;
pub mod outcome;
pub mod prediction;
pub mod report;
