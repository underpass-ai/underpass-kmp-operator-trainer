//! Input DTO for `BuildTrainingRunUseCase::execute`. Groups every
//! non-port input the use case needs so the caller does not have to
//! thread six positional arguments through.

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_training_domain::ids::TrainingRunId;
use operator_training_domain::provenance::dataset_source::DatasetSource;
use operator_training_domain::readiness::readiness_gate::ReadinessGate;
use operator_training_domain::trainer::trainer_target::TrainerTarget;

#[derive(Debug, Clone, PartialEq)]
pub struct BuildTrainingRunRequest {
    run_id: TrainingRunId,
    trainer_target: TrainerTarget,
    dataset_source: DatasetSource,
    gates: Vec<ReadinessGate>,
    evaluation: EvaluationReport,
}

impl BuildTrainingRunRequest {
    pub fn new(
        run_id: TrainingRunId,
        trainer_target: TrainerTarget,
        dataset_source: DatasetSource,
        gates: Vec<ReadinessGate>,
        evaluation: EvaluationReport,
    ) -> Self {
        Self {
            run_id,
            trainer_target,
            dataset_source,
            gates,
            evaluation,
        }
    }

    pub fn run_id(&self) -> &TrainingRunId {
        &self.run_id
    }

    pub fn trainer_target(&self) -> &TrainerTarget {
        &self.trainer_target
    }

    pub fn dataset_source(&self) -> &DatasetSource {
        &self.dataset_source
    }

    pub fn gates(&self) -> &[ReadinessGate] {
        &self.gates
    }

    pub fn evaluation(&self) -> &EvaluationReport {
        &self.evaluation
    }
}
