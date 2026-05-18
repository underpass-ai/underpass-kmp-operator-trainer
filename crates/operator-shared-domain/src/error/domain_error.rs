//! Domain error variants for the Operator shared bounded context.
//!
//! Every constructor that refuses to build an invalid value returns one of
//! the variants below. Higher-level layers (`application`, `infra`) wrap
//! these into application or infrastructure errors.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("{context} is empty")]
    EmptyValue { context: &'static str },

    #[error("{context} count must be greater than zero")]
    NonPositiveCount { context: &'static str },

    #[error("{context} count {actual} is below required minimum {minimum}")]
    CountBelowMinimum {
        context: &'static str,
        minimum: usize,
        actual: usize,
    },

    #[error("tool {tool} is not allowed in mode {mode}")]
    ToolOutsideMode {
        mode: &'static str,
        tool: &'static str,
    },

    #[error("trajectory invariant {field} mismatch: expected {expected}, got {actual}")]
    TrajectoryInvariantMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },

    #[error("visible state does not contain referenced {kind} '{value}'")]
    VisibleStateMissingReference { kind: &'static str, value: String },

    #[error("unsupported value '{value}' for {context}")]
    UnsupportedValue {
        context: &'static str,
        value: String,
    },
}
