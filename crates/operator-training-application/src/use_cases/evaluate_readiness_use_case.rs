//! Evaluate each `ReadinessGate` against a `DatasetProvenance` plus
//! an `EvaluationReport`, then assemble the per-gate checks into a
//! `ReadinessReport`. The use case is pure: no ports, no I/O.

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_training_domain::errors::training_result::TrainingResult;
use operator_training_domain::provenance::dataset_provenance::DatasetProvenance;
use operator_training_domain::readiness::readiness_gate::ReadinessGate;
use operator_training_domain::readiness::readiness_report::ReadinessReport;

#[derive(Debug)]
pub struct EvaluateReadinessUseCase;

impl EvaluateReadinessUseCase {
    /// Run every gate against the provenance + evaluation report and
    /// build the resulting report. Refuses an empty gate list (an
    /// empty readiness report is a configuration smell, enforced at
    /// the domain level).
    pub fn execute(
        gates: &[ReadinessGate],
        provenance: &DatasetProvenance,
        evaluation: &EvaluationReport,
    ) -> TrainingResult<ReadinessReport> {
        let checks = gates
            .iter()
            .map(|gate| gate.evaluate(provenance, evaluation))
            .collect();
        ReadinessReport::new(checks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_training_domain::errors::training_domain_error::TrainingDomainError;
    use operator_training_domain::provenance::content_hash::ContentHash;
    use operator_training_domain::provenance::dataset_source::DatasetSource;
    use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
    use operator_training_domain::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
    use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;

    fn provenance_with_count(count: usize, families: &[(&str, usize)]) -> DatasetProvenance {
        let entries = families
            .iter()
            .map(|(f, c)| {
                TaskFamilyDistributionEntry::new(
                    TaskFamily::parse(*f).unwrap(),
                    PositiveCount::parse(*c, "test").unwrap(),
                )
            })
            .collect();
        DatasetProvenance::new(
            DatasetSource::parse("src").unwrap(),
            ContentHash::parse("sha256:x").unwrap(),
            PositiveCount::parse(count, "trajectory_count").unwrap(),
            TaskFamilyDistribution::new(entries).unwrap(),
        )
        .unwrap()
    }

    fn empty_eval() -> EvaluationReport {
        EvaluationReport::from_outcomes(Vec::new())
    }

    #[test]
    fn refuses_empty_gate_list() {
        let p = provenance_with_count(1, &[("read.inspect", 1)]);
        let err = EvaluateReadinessUseCase::execute(&[], &p, &empty_eval()).unwrap_err();
        assert!(matches!(err, TrainingDomainError::EmptyReadinessReport));
    }

    #[test]
    fn collects_one_check_per_gate() {
        let gates = vec![
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
            ReadinessGate::MinimumTaskFamilyCoverage(PositiveCount::parse(1, "x").unwrap()),
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.0).unwrap()),
        ];
        let p = provenance_with_count(1, &[("read.inspect", 1)]);
        let report = EvaluateReadinessUseCase::execute(&gates, &p, &empty_eval()).unwrap();
        assert_eq!(report.total(), 3);
        assert!(report.is_ready());
    }

    #[test]
    fn surface_failed_gates_in_the_report() {
        let gates = vec![
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(10, "x").unwrap()),
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.9).unwrap()),
        ];
        let p = provenance_with_count(1, &[("read.inspect", 1)]);
        let report = EvaluateReadinessUseCase::execute(&gates, &p, &empty_eval()).unwrap();
        assert!(!report.is_ready());
        assert_eq!(report.failed_count(), 2);
    }
}
