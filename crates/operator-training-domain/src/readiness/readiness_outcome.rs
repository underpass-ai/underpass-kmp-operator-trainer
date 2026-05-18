//! Per-gate verdict carried by a `ReadinessCheck`. `Passed` is unitary;
//! `Failed` keeps a human-readable reason so the manifest captures why
//! the gate refused without forcing every consumer to re-evaluate.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadinessOutcome {
    Passed,
    Failed { reason: NonEmptyString },
}

impl ReadinessOutcome {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}
