//! Aggregated readiness assessment. `is_ready()` is true iff every
//! check on the report passed. Refuses to be built from an empty
//! gate list: a run with zero gates is a configuration smell, not a
//! healthy default.
//!
//! ## Verdict surface
//!
//! `is_ready()` is the single verdict surface today. Callers that
//! want to pass the verdict around without the full report (e.g.,
//! to log it as a flat enum, to serialise it separately, or to use
//! it as a routing key) should reify a `ReadinessVerdict` value
//! object alongside this aggregate. That refactor is intentionally
//! deferred until a real second consumer surfaces — adding it now
//! would be speculative API growth, and `is_ready()` is the same
//! semantics anyway.

use crate::errors::training_domain_error::TrainingDomainError;
use crate::errors::training_result::TrainingResult;
use crate::readiness::readiness_check::ReadinessCheck;

#[derive(Debug, Clone, PartialEq)]
pub struct ReadinessReport {
    checks: Vec<ReadinessCheck>,
}

impl ReadinessReport {
    pub fn new(checks: Vec<ReadinessCheck>) -> TrainingResult<Self> {
        if checks.is_empty() {
            return Err(TrainingDomainError::EmptyReadinessReport);
        }
        Ok(Self { checks })
    }

    pub fn checks(&self) -> &[ReadinessCheck] {
        &self.checks
    }

    pub fn is_ready(&self) -> bool {
        self.checks.iter().all(ReadinessCheck::is_passed)
    }

    pub fn failed_count(&self) -> usize {
        self.checks.iter().filter(|c| !c.is_passed()).count()
    }

    pub fn total(&self) -> usize {
        self.checks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readiness::pass_rate_percent::PassRatePercent;
    use crate::readiness::readiness_gate::ReadinessGate;
    use crate::readiness::readiness_outcome::ReadinessOutcome;
    use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    fn passed_check() -> ReadinessCheck {
        ReadinessCheck::new(
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
            ReadinessOutcome::Passed,
        )
    }

    fn failed_check() -> ReadinessCheck {
        ReadinessCheck::new(
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.9).unwrap()),
            ReadinessOutcome::Failed {
                reason: NonEmptyString::parse("rate too low", "test").unwrap(),
            },
        )
    }

    #[test]
    fn refuses_empty_report() {
        assert!(matches!(
            ReadinessReport::new(vec![]),
            Err(TrainingDomainError::EmptyReadinessReport)
        ));
    }

    #[test]
    fn all_passed_is_ready() {
        let report = ReadinessReport::new(vec![passed_check(), passed_check()]).unwrap();
        assert!(report.is_ready());
        assert_eq!(report.failed_count(), 0);
        assert_eq!(report.total(), 2);
    }

    #[test]
    fn any_failed_is_not_ready() {
        let report = ReadinessReport::new(vec![passed_check(), failed_check()]).unwrap();
        assert!(!report.is_ready());
        assert_eq!(report.failed_count(), 1);
        assert_eq!(report.total(), 2);
    }
}
