//! Invoke the external trainer for a `TrainingRun`. The run aggregate
//! root guarantees readiness, so this use case has nothing to check
//! domain-side; it just hands the manifest's `TrainerTarget` to the
//! injected `TrainerInvoker` port and surfaces the typed outcome.

use operator_training_domain::run::training_run::TrainingRun;

use crate::errors::training_application_result::TrainingApplicationResult;
use crate::ports::trainer_invocation_outcome::TrainerInvocationOutcome;
use crate::ports::trainer_invoker::TrainerInvoker;

#[derive(Debug)]
pub struct LaunchTrainingRunUseCase<I: TrainerInvoker> {
    invoker: I,
}

impl<I: TrainerInvoker> LaunchTrainingRunUseCase<I> {
    pub fn new(invoker: I) -> Self {
        Self { invoker }
    }

    pub fn execute(
        &self,
        run: &TrainingRun,
    ) -> TrainingApplicationResult<TrainerInvocationOutcome> {
        let outcome = self.invoker.invoke(run.manifest().trainer_target())?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_training_domain::ids::TrainingRunId;
    use operator_training_domain::manifest::training_manifest::TrainingManifest;
    use operator_training_domain::provenance::content_hash::ContentHash;
    use operator_training_domain::provenance::dataset_provenance::DatasetProvenance;
    use operator_training_domain::provenance::dataset_source::DatasetSource;
    use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
    use operator_training_domain::readiness::readiness_check::ReadinessCheck;
    use operator_training_domain::readiness::readiness_gate::ReadinessGate;
    use operator_training_domain::readiness::readiness_outcome::ReadinessOutcome;
    use operator_training_domain::readiness::readiness_report::ReadinessReport;
    use operator_training_domain::trainer::base_model_id::BaseModelId;
    use operator_training_domain::trainer::output_directory::OutputDirectory;
    use operator_training_domain::trainer::trainer_command::TrainerCommand;
    use operator_training_domain::trainer::trainer_target::TrainerTarget;
    use std::sync::Mutex;

    use crate::errors::trainer_invoker_error::TrainerInvokerError;
    use crate::errors::training_application_error::TrainingApplicationError;

    #[derive(Debug)]
    struct RecordingInvoker {
        invocations: Mutex<Vec<String>>,
        outcome: TrainerInvocationOutcome,
    }

    impl TrainerInvoker for RecordingInvoker {
        fn invoke(
            &self,
            target: &TrainerTarget,
        ) -> Result<TrainerInvocationOutcome, TrainerInvokerError> {
            self.invocations
                .lock()
                .unwrap()
                .push(target.command().as_str().to_string());
            Ok(self.outcome.clone())
        }
    }

    #[derive(Debug)]
    struct FailingInvoker;

    impl TrainerInvoker for FailingInvoker {
        fn invoke(
            &self,
            target: &TrainerTarget,
        ) -> Result<TrainerInvocationOutcome, TrainerInvokerError> {
            Err(TrainerInvokerError::SpawnFailure {
                adapter: "stub",
                command: target.command().as_str().to_string(),
                message: "not found".into(),
            })
        }
    }

    fn ready_run() -> TrainingRun {
        let id = TrainingRunId::parse("run:1").unwrap();
        let provenance = DatasetProvenance::new(
            DatasetSource::parse("src").unwrap(),
            ContentHash::parse("sha256:abc").unwrap(),
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
        let manifest = TrainingManifest::new(id.clone(), provenance, trainer_target, readiness);
        TrainingRun::new(id, manifest).unwrap()
    }

    #[test]
    fn forwards_trainer_target_to_invoker_and_returns_outcome() {
        let use_case = LaunchTrainingRunUseCase::new(RecordingInvoker {
            invocations: Mutex::new(Vec::new()),
            outcome: TrainerInvocationOutcome::Success { exit_code: 0 },
        });
        let run = ready_run();
        let outcome = use_case.execute(&run).unwrap();
        assert!(outcome.is_success());
        assert_eq!(
            use_case.invoker.invocations.lock().unwrap().as_slice(),
            &["sft-trainer".to_string()],
        );
    }

    #[test]
    fn invoker_failure_bubbles_out() {
        let use_case = LaunchTrainingRunUseCase::new(FailingInvoker);
        let run = ready_run();
        let err = use_case.execute(&run).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::TrainerInvoker(TrainerInvokerError::SpawnFailure { .. })
        ));
    }
}
