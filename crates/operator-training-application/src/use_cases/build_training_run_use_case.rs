//! Top-level orchestration for the training context. Reads
//! trajectories from a `TrajectorySource`, writes them via a
//! `DatasetWriter`, computes the dataset provenance from the writer's
//! outcome, evaluates the readiness gates against the supplied
//! `EvaluationReport`, assembles a `TrainingManifest`, writes it
//! through a `ManifestWriter`, and finally hands the manifest to
//! `TrainingRun::new`, which refuses to build a run whose manifest
//! failed readiness.
//!
//! The manifest is written **before** `TrainingRun::new` is called:
//! callers want the unready manifest on disk for diagnostics even
//! when the run itself cannot launch.

use operator_training_domain::provenance::dataset_provenance::DatasetProvenance;
use operator_training_domain::run::training_run::TrainingRun;

use crate::errors::training_application_result::TrainingApplicationResult;
use crate::ports::dataset_writer::DatasetWriter;
use crate::ports::manifest_writer::ManifestWriter;
use crate::ports::trajectory_source::TrajectorySource;
use crate::use_cases::assemble_manifest_use_case::AssembleManifestUseCase;
use crate::use_cases::build_training_run_request::BuildTrainingRunRequest;
use crate::use_cases::evaluate_readiness_use_case::EvaluateReadinessUseCase;

#[derive(Debug)]
pub struct BuildTrainingRunUseCase<S: TrajectorySource, D: DatasetWriter, M: ManifestWriter> {
    trajectory_source: S,
    dataset_writer: D,
    manifest_writer: M,
}

impl<S, D, M> BuildTrainingRunUseCase<S, D, M>
where
    S: TrajectorySource,
    D: DatasetWriter,
    M: ManifestWriter,
{
    pub fn new(trajectory_source: S, dataset_writer: D, manifest_writer: M) -> Self {
        Self {
            trajectory_source,
            dataset_writer,
            manifest_writer,
        }
    }

    pub fn execute(
        &self,
        request: &BuildTrainingRunRequest,
    ) -> TrainingApplicationResult<TrainingRun> {
        let trajectories = self.trajectory_source.fetch_all()?;
        let write_outcome = self.dataset_writer.write(&trajectories)?;
        let provenance = DatasetProvenance::new(
            request.dataset_source().clone(),
            write_outcome.content_hash().clone(),
            write_outcome.trajectory_count(),
            write_outcome.distribution().clone(),
        )?;
        let readiness =
            EvaluateReadinessUseCase::execute(request.gates(), &provenance, request.evaluation())?;
        let manifest = AssembleManifestUseCase::execute(
            request.run_id().clone(),
            provenance,
            request.trainer_target().clone(),
            readiness,
        );
        self.manifest_writer.write(&manifest)?;
        Ok(TrainingRun::new(request.run_id().clone(), manifest)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_training_domain::errors::training_domain_error::TrainingDomainError;
    use operator_training_domain::ids::TrainingRunId;
    use operator_training_domain::provenance::content_hash::ContentHash;
    use operator_training_domain::provenance::dataset_source::DatasetSource;
    use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
    use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;
    use operator_training_domain::readiness::readiness_gate::ReadinessGate;
    use operator_training_domain::trainer::base_model_id::BaseModelId;
    use operator_training_domain::trainer::output_directory::OutputDirectory;
    use operator_training_domain::trainer::trainer_command::TrainerCommand;
    use operator_training_domain::trainer::trainer_target::TrainerTarget;
    use std::sync::Mutex;

    use crate::errors::dataset_write_error::DatasetWriteError;
    use crate::errors::manifest_write_error::ManifestWriteError;
    use crate::errors::training_application_error::TrainingApplicationError;
    use crate::errors::trajectory_source_error::TrajectorySourceError;
    use crate::ports::dataset_write_outcome::DatasetWriteOutcome;

    #[derive(Debug)]
    struct StubTrajectorySource {
        trajectories: Vec<TrainingTrajectory>,
    }

    impl TrajectorySource for StubTrajectorySource {
        fn fetch_all(&self) -> Result<Vec<TrainingTrajectory>, TrajectorySourceError> {
            Ok(self.trajectories.clone())
        }
    }

    #[derive(Debug)]
    struct StubFailingTrajectorySource;

    impl TrajectorySource for StubFailingTrajectorySource {
        fn fetch_all(&self) -> Result<Vec<TrainingTrajectory>, TrajectorySourceError> {
            Err(TrajectorySourceError::Unavailable {
                adapter: "stub",
                message: "no data".into(),
            })
        }
    }

    #[derive(Debug)]
    struct StubDatasetWriter {
        outcome: DatasetWriteOutcome,
    }

    impl DatasetWriter for StubDatasetWriter {
        fn write(
            &self,
            _trajectories: &[TrainingTrajectory],
        ) -> Result<DatasetWriteOutcome, DatasetWriteError> {
            Ok(self.outcome.clone())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingManifestWriter {
        writes: Mutex<usize>,
    }

    impl ManifestWriter for RecordingManifestWriter {
        fn write(
            &self,
            _manifest: &operator_training_domain::manifest::training_manifest::TrainingManifest,
        ) -> Result<(), ManifestWriteError> {
            *self.writes.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FailingManifestWriter;

    impl ManifestWriter for FailingManifestWriter {
        fn write(
            &self,
            _manifest: &operator_training_domain::manifest::training_manifest::TrainingManifest,
        ) -> Result<(), ManifestWriteError> {
            Err(ManifestWriteError::WriteFailure {
                adapter: "stub",
                message: "disk full".into(),
            })
        }
    }

    fn inspect_trajectory() -> TrainingTrajectory {
        let target = MemoryRef::parse("node:1").unwrap();
        let visible = VisibleState::assemble(
            [target.clone()],
            std::iter::empty(),
            None,
            BudgetSnapshot::unbounded(),
        );
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse("traj:1").unwrap(),
            StepId::parse("step:1").unwrap(),
            AboutId::parse("about:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Inspect the visible memory node.",
            )
            .unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
        .unwrap()
    }

    fn write_outcome(count: usize) -> DatasetWriteOutcome {
        DatasetWriteOutcome::new(
            ContentHash::parse("sha256:abc").unwrap(),
            PositiveCount::parse(count, "trajectory_count").unwrap(),
            TaskFamilyDistribution::new(vec![]).unwrap(),
        )
    }

    fn request_with_gates(gates: Vec<ReadinessGate>) -> BuildTrainingRunRequest {
        BuildTrainingRunRequest::new(
            TrainingRunId::parse("run:1").unwrap(),
            TrainerTarget::new(
                TrainerCommand::parse("sft-trainer").unwrap(),
                BaseModelId::parse("base").unwrap(),
                OutputDirectory::parse("out").unwrap(),
            ),
            DatasetSource::parse("synthetic-run:test").unwrap(),
            gates,
            EvaluationReport::from_outcomes(Vec::new()),
        )
    }

    #[test]
    fn happy_path_builds_a_ready_run() {
        let use_case = BuildTrainingRunUseCase::new(
            StubTrajectorySource {
                trajectories: vec![inspect_trajectory()],
            },
            StubDatasetWriter {
                outcome: write_outcome(1),
            },
            RecordingManifestWriter::default(),
        );

        let request = request_with_gates(vec![ReadinessGate::MinimumTrajectories(
            PositiveCount::parse(1, "x").unwrap(),
        )]);
        let run = use_case.execute(&request).expect("happy path");

        assert_eq!(run.id().as_str(), "run:1");
        assert!(run.manifest().readiness().is_ready());
        assert_eq!(
            *use_case.manifest_writer.writes.lock().unwrap(),
            1,
            "manifest written exactly once on the happy path"
        );
    }

    #[test]
    fn trajectory_source_failure_bubbles_out() {
        let use_case = BuildTrainingRunUseCase::new(
            StubFailingTrajectorySource,
            StubDatasetWriter {
                outcome: write_outcome(1),
            },
            RecordingManifestWriter::default(),
        );
        let request = request_with_gates(vec![ReadinessGate::MinimumTrajectories(
            PositiveCount::parse(1, "x").unwrap(),
        )]);
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::TrajectorySource(TrajectorySourceError::Unavailable { .. })
        ));
        assert_eq!(
            *use_case.manifest_writer.writes.lock().unwrap(),
            0,
            "manifest not written when fetch fails"
        );
    }

    #[test]
    fn manifest_writer_failure_bubbles_out() {
        let use_case = BuildTrainingRunUseCase::new(
            StubTrajectorySource {
                trajectories: vec![inspect_trajectory()],
            },
            StubDatasetWriter {
                outcome: write_outcome(1),
            },
            FailingManifestWriter,
        );
        let request = request_with_gates(vec![ReadinessGate::MinimumTrajectories(
            PositiveCount::parse(1, "x").unwrap(),
        )]);
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::ManifestWrite(ManifestWriteError::WriteFailure { .. })
        ));
    }

    #[test]
    fn unready_manifest_writes_disk_then_refuses_run() {
        let use_case = BuildTrainingRunUseCase::new(
            StubTrajectorySource {
                trajectories: vec![inspect_trajectory()],
            },
            StubDatasetWriter {
                outcome: write_outcome(1),
            },
            RecordingManifestWriter::default(),
        );
        // Threshold 10 but only 1 trajectory → gate fails.
        let request = request_with_gates(vec![
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(10, "x").unwrap()),
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.9).unwrap()),
        ]);
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::Domain(TrainingDomainError::NotReady { .. })
        ));
        assert_eq!(
            *use_case.manifest_writer.writes.lock().unwrap(),
            1,
            "unready manifest must still be persisted for diagnostics"
        );
    }
}
