//! A single gate evaluation: the gate that was checked plus its
//! outcome (passed or failed-with-reason).

use crate::readiness::readiness_gate::ReadinessGate;
use crate::readiness::readiness_outcome::ReadinessOutcome;

#[derive(Debug, Clone, PartialEq)]
pub struct ReadinessCheck {
    gate: ReadinessGate,
    outcome: ReadinessOutcome,
}

impl ReadinessCheck {
    pub fn new(gate: ReadinessGate, outcome: ReadinessOutcome) -> Self {
        Self { gate, outcome }
    }

    pub fn gate(&self) -> &ReadinessGate {
        &self.gate
    }

    pub fn outcome(&self) -> &ReadinessOutcome {
        &self.outcome
    }

    pub fn is_passed(&self) -> bool {
        self.outcome.is_passed()
    }
}
