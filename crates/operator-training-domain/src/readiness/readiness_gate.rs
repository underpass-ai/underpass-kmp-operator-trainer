//! Closed enum of readiness gates evaluated before a `TrainingRun`
//! is allowed to launch. Each variant carries its target threshold;
//! `evaluate` produces a `ReadinessCheck` against a `DatasetProvenance`
//! plus an `EvaluationReport`.
//!
//! Adding a new gate is a deliberate domain change: every exhaustive
//! match over `ReadinessGate` breaks at compile time, by design.
//! Wildcard matches (`_ => ...`) on `ReadinessGate` are forbidden by
//! convention and reviewed at PR time.

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::provenance::dataset_provenance::DatasetProvenance;
use crate::readiness::pass_rate_percent::PassRatePercent;
use crate::readiness::readiness_check::ReadinessCheck;
use crate::readiness::readiness_outcome::ReadinessOutcome;

#[derive(Debug, Clone, PartialEq)]
pub enum ReadinessGate {
    /// Dataset must contain at least N trajectories.
    MinimumTrajectories(PositiveCount),

    /// Dataset must cover at least N distinct task families.
    MinimumTaskFamilyCoverage(PositiveCount),

    /// Exact-match rate on the evaluation report must be ≥ target.
    MinimumEvaluationPassRate(PassRatePercent),
}

impl ReadinessGate {
    /// Evaluate the gate against the dataset provenance and the
    /// evaluation report it was scored on. Returns a `ReadinessCheck`
    /// that records both the gate's target and the outcome.
    pub fn evaluate(
        &self,
        provenance: &DatasetProvenance,
        evaluation: &EvaluationReport,
    ) -> ReadinessCheck {
        let outcome = match self {
            Self::MinimumTrajectories(target) => evaluate_trajectory_count(*target, provenance),
            Self::MinimumTaskFamilyCoverage(target) => {
                evaluate_family_coverage(*target, provenance)
            }
            Self::MinimumEvaluationPassRate(target) => evaluate_pass_rate(*target, evaluation),
        };
        ReadinessCheck::new(self.clone(), outcome)
    }
}

fn evaluate_trajectory_count(
    target: PositiveCount,
    provenance: &DatasetProvenance,
) -> ReadinessOutcome {
    let actual = provenance.trajectory_count().as_usize();
    let target_n = target.as_usize();
    if actual >= target_n {
        ReadinessOutcome::Passed
    } else {
        ReadinessOutcome::Failed {
            reason: reason(format!("trajectory_count {actual} < target {target_n}")),
        }
    }
}

fn evaluate_family_coverage(
    target: PositiveCount,
    provenance: &DatasetProvenance,
) -> ReadinessOutcome {
    let actual = provenance.distribution().family_count();
    let target_n = target.as_usize();
    if actual >= target_n {
        ReadinessOutcome::Passed
    } else {
        ReadinessOutcome::Failed {
            reason: reason(format!("task family coverage {actual} < target {target_n}")),
        }
    }
}

fn evaluate_pass_rate(target: PassRatePercent, evaluation: &EvaluationReport) -> ReadinessOutcome {
    let actual = evaluation.exact_match_rate();
    if actual >= target.as_f64() {
        ReadinessOutcome::Passed
    } else {
        ReadinessOutcome::Failed {
            reason: reason(format!("exact_match_rate {actual:.4} < target {target}")),
        }
    }
}

fn reason(text: String) -> NonEmptyString {
    NonEmptyString::parse(text, "readiness_outcome.reason")
        .expect("non-empty readiness reason text")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::content_hash::ContentHash;
    use crate::provenance::dataset_source::DatasetSource;
    use crate::provenance::task_family_distribution::TaskFamilyDistribution;
    use crate::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
    use operator_shared_domain::value_objects::task_family::TaskFamily;

    fn distribution(entries: &[(&str, usize)]) -> TaskFamilyDistribution {
        TaskFamilyDistribution::new(
            entries
                .iter()
                .map(|(f, c)| {
                    TaskFamilyDistributionEntry::new(
                        TaskFamily::parse(*f).unwrap(),
                        PositiveCount::parse(*c, "test").unwrap(),
                    )
                })
                .collect(),
        )
        .unwrap()
    }

    fn provenance_of(trajectory_count: usize, dist: &[(&str, usize)]) -> DatasetProvenance {
        DatasetProvenance::new(
            DatasetSource::parse("synthetic-run:test").unwrap(),
            ContentHash::parse("sha256:abc").unwrap(),
            PositiveCount::parse(trajectory_count, "trajectory_count").unwrap(),
            distribution(dist),
        )
        .unwrap()
    }

    fn empty_evaluation() -> EvaluationReport {
        EvaluationReport::from_outcomes(Vec::new())
    }

    #[test]
    fn minimum_trajectories_passes_when_at_or_above_target() {
        let gate = ReadinessGate::MinimumTrajectories(PositiveCount::parse(10, "x").unwrap());
        let p = provenance_of(10, &[("read.inspect", 10)]);
        assert!(gate.evaluate(&p, &empty_evaluation()).is_passed());
    }

    #[test]
    fn minimum_trajectories_fails_when_below_target() {
        let gate = ReadinessGate::MinimumTrajectories(PositiveCount::parse(11, "x").unwrap());
        let p = provenance_of(10, &[("read.inspect", 10)]);
        let check = gate.evaluate(&p, &empty_evaluation());
        assert!(!check.is_passed());
    }

    #[test]
    fn minimum_task_family_coverage_passes_when_count_meets_target() {
        let gate = ReadinessGate::MinimumTaskFamilyCoverage(PositiveCount::parse(2, "x").unwrap());
        let p = provenance_of(10, &[("read.inspect", 6), ("read.ask", 4)]);
        assert!(gate.evaluate(&p, &empty_evaluation()).is_passed());
    }

    #[test]
    fn minimum_task_family_coverage_fails_when_count_below_target() {
        let gate = ReadinessGate::MinimumTaskFamilyCoverage(PositiveCount::parse(3, "x").unwrap());
        let p = provenance_of(10, &[("read.inspect", 6), ("read.ask", 4)]);
        assert!(!gate.evaluate(&p, &empty_evaluation()).is_passed());
    }

    #[test]
    fn minimum_pass_rate_uses_exact_match_rate_from_report() {
        // empty report yields rate 0.0 → fails any positive threshold
        let gate = ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.5).unwrap());
        let p = provenance_of(10, &[("read.inspect", 10)]);
        assert!(!gate.evaluate(&p, &empty_evaluation()).is_passed());
    }

    #[test]
    fn minimum_pass_rate_passes_when_target_is_zero() {
        let gate = ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.0).unwrap());
        let p = provenance_of(10, &[("read.inspect", 10)]);
        assert!(gate.evaluate(&p, &empty_evaluation()).is_passed());
    }
}
