//! Typed result of reading a predictions source. Parsed predictions
//! can be scored normally; recoverable shape violations are counted
//! as evaluated invalid rows.

use crate::prediction::shape_violation_record::ShapeViolationRecord;
use crate::prediction::step_keyed_prediction::StepKeyedPrediction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionsReadOutcome {
    parsed: Vec<StepKeyedPrediction>,
    shape_violations: Vec<ShapeViolationRecord>,
}

impl PredictionsReadOutcome {
    pub fn new(
        parsed: Vec<StepKeyedPrediction>,
        shape_violations: Vec<ShapeViolationRecord>,
    ) -> Self {
        Self {
            parsed,
            shape_violations,
        }
    }

    pub fn parsed(&self) -> &[StepKeyedPrediction] {
        &self.parsed
    }

    pub fn shape_violations(&self) -> &[ShapeViolationRecord] {
        &self.shape_violations
    }

    pub fn into_parts(self) -> (Vec<StepKeyedPrediction>, Vec<ShapeViolationRecord>) {
        (self.parsed, self.shape_violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_parsed_and_violations() {
        let violation = ShapeViolationRecord::new(3, None, "bad action").unwrap();
        let outcome = PredictionsReadOutcome::new(Vec::new(), vec![violation.clone()]);

        assert!(outcome.parsed().is_empty());
        assert_eq!(outcome.shape_violations(), &[violation]);
    }
}
