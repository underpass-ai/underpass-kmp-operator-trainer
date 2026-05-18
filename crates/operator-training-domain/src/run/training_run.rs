//! `TrainingRun` is the aggregate root of the training context. A
//! value of this type represents a run that is **allowed to launch**:
//! its named constructor refuses to build a `TrainingRun` whose
//! manifest is not ready, and refuses to build one whose manifest
//! identifies a different run.

use crate::errors::training_domain_error::TrainingDomainError;
use crate::errors::training_result::TrainingResult;
use crate::ids::TrainingRunId;
use crate::manifest::training_manifest::TrainingManifest;

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingRun {
    id: TrainingRunId,
    manifest: TrainingManifest,
}

impl TrainingRun {
    pub fn new(id: TrainingRunId, manifest: TrainingManifest) -> TrainingResult<Self> {
        if manifest.run_id() != &id {
            return Err(TrainingDomainError::ManifestRunIdMismatch {
                manifest: manifest.run_id().as_str().to_string(),
                run: id.as_str().to_string(),
            });
        }
        let readiness = manifest.readiness();
        if !readiness.is_ready() {
            return Err(TrainingDomainError::NotReady {
                failed: readiness.failed_count(),
                total: readiness.total(),
                summary: summarise_failures(readiness),
            });
        }
        Ok(Self { id, manifest })
    }

    pub fn id(&self) -> &TrainingRunId {
        &self.id
    }

    pub fn manifest(&self) -> &TrainingManifest {
        &self.manifest
    }
}

fn summarise_failures(readiness: &crate::readiness::readiness_report::ReadinessReport) -> String {
    use crate::readiness::readiness_outcome::ReadinessOutcome;
    readiness
        .checks()
        .iter()
        .filter_map(|check| match check.outcome() {
            ReadinessOutcome::Failed { reason } => Some(reason.as_str().to_string()),
            ReadinessOutcome::Passed => None,
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::content_hash::ContentHash;
    use crate::provenance::dataset_provenance::DatasetProvenance;
    use crate::provenance::dataset_source::DatasetSource;
    use crate::provenance::task_family_distribution::TaskFamilyDistribution;
    use crate::readiness::pass_rate_percent::PassRatePercent;
    use crate::readiness::readiness_check::ReadinessCheck;
    use crate::readiness::readiness_gate::ReadinessGate;
    use crate::readiness::readiness_outcome::ReadinessOutcome;
    use crate::readiness::readiness_report::ReadinessReport;
    use crate::trainer::base_model_id::BaseModelId;
    use crate::trainer::output_directory::OutputDirectory;
    use crate::trainer::trainer_command::TrainerCommand;
    use crate::trainer::trainer_target::TrainerTarget;
    use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    fn run_id(value: &str) -> TrainingRunId {
        TrainingRunId::parse(value).unwrap()
    }

    fn provenance() -> DatasetProvenance {
        DatasetProvenance::new(
            DatasetSource::parse("synthetic-run:test").unwrap(),
            ContentHash::parse("sha256:abc").unwrap(),
            PositiveCount::parse(10, "trajectory_count").unwrap(),
            TaskFamilyDistribution::new(vec![]).unwrap(),
        )
        .unwrap()
    }

    fn trainer_target() -> TrainerTarget {
        TrainerTarget::new(
            TrainerCommand::parse("sft-trainer").unwrap(),
            BaseModelId::parse("base").unwrap(),
            OutputDirectory::parse("out").unwrap(),
        )
    }

    fn passing_report() -> ReadinessReport {
        ReadinessReport::new(vec![ReadinessCheck::new(
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
            ReadinessOutcome::Passed,
        )])
        .unwrap()
    }

    fn failing_report() -> ReadinessReport {
        ReadinessReport::new(vec![ReadinessCheck::new(
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.9).unwrap()),
            ReadinessOutcome::Failed {
                reason: NonEmptyString::parse("rate too low", "test").unwrap(),
            },
        )])
        .unwrap()
    }

    fn manifest_with(id: &TrainingRunId, readiness: ReadinessReport) -> TrainingManifest {
        TrainingManifest::new(id.clone(), provenance(), trainer_target(), readiness)
    }

    #[test]
    fn refuses_unready_manifest() {
        let id = run_id("run:1");
        let manifest = manifest_with(&id, failing_report());
        let err = TrainingRun::new(id, manifest).unwrap_err();
        assert!(matches!(
            err,
            TrainingDomainError::NotReady {
                failed: 1,
                total: 1,
                ..
            }
        ));
    }

    #[test]
    fn refuses_manifest_with_different_run_id() {
        let id = run_id("run:1");
        let other = run_id("run:2");
        let manifest = manifest_with(&other, passing_report());
        let err = TrainingRun::new(id, manifest).unwrap_err();
        assert!(matches!(
            err,
            TrainingDomainError::ManifestRunIdMismatch { .. }
        ));
    }

    #[test]
    fn builds_when_manifest_is_ready_and_ids_match() {
        let id = run_id("run:1");
        let manifest = manifest_with(&id, passing_report());
        let run = TrainingRun::new(id.clone(), manifest).expect("ready run must build");
        assert_eq!(run.id(), &id);
        assert!(run.manifest().readiness().is_ready());
    }
}
