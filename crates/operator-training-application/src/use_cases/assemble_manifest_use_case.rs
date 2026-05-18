//! Compose a `TrainingManifest` from its parts. Pure orchestration —
//! the manifest itself is representable in any readiness state, so
//! this use case never fails. It exists to make the assembly contract
//! explicit and to centralise where the manifest is built.

use operator_training_domain::ids::TrainingRunId;
use operator_training_domain::manifest::training_manifest::TrainingManifest;
use operator_training_domain::provenance::dataset_provenance::DatasetProvenance;
use operator_training_domain::readiness::readiness_report::ReadinessReport;
use operator_training_domain::trainer::trainer_target::TrainerTarget;

#[derive(Debug)]
pub struct AssembleManifestUseCase;

impl AssembleManifestUseCase {
    pub fn execute(
        run_id: TrainingRunId,
        provenance: DatasetProvenance,
        trainer_target: TrainerTarget,
        readiness: ReadinessReport,
    ) -> TrainingManifest {
        TrainingManifest::new(run_id, provenance, trainer_target, readiness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_training_domain::provenance::content_hash::ContentHash;
    use operator_training_domain::provenance::dataset_source::DatasetSource;
    use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
    use operator_training_domain::readiness::readiness_check::ReadinessCheck;
    use operator_training_domain::readiness::readiness_gate::ReadinessGate;
    use operator_training_domain::readiness::readiness_outcome::ReadinessOutcome;
    use operator_training_domain::trainer::base_model_id::BaseModelId;
    use operator_training_domain::trainer::output_directory::OutputDirectory;
    use operator_training_domain::trainer::trainer_command::TrainerCommand;

    #[test]
    fn assembles_manifest_with_inputs_intact() {
        let run_id = TrainingRunId::parse("run:1").unwrap();
        let provenance = DatasetProvenance::new(
            DatasetSource::parse("src").unwrap(),
            ContentHash::parse("sha256:x").unwrap(),
            PositiveCount::parse(1, "trajectory_count").unwrap(),
            TaskFamilyDistribution::new(vec![]).unwrap(),
        )
        .unwrap();
        let trainer_target = TrainerTarget::new(
            TrainerCommand::parse("sft-trainer").unwrap(),
            BaseModelId::parse("base").unwrap(),
            OutputDirectory::parse("out").unwrap(),
        );
        let readiness = ReadinessReport::new(vec![ReadinessCheck::new(
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
            ReadinessOutcome::Passed,
        )])
        .unwrap();

        let manifest =
            AssembleManifestUseCase::execute(run_id.clone(), provenance, trainer_target, readiness);

        assert_eq!(manifest.run_id(), &run_id);
        assert_eq!(manifest.dataset().source().as_str(), "src");
        assert_eq!(manifest.trainer_target().command().as_str(), "sft-trainer");
        assert!(manifest.readiness().is_ready());
    }
}
