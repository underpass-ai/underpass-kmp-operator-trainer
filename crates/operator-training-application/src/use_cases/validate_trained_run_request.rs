//! Input DTO for `ValidateTrainedRunUseCase::execute`. Carries the
//! predictor target (where the `LoRA` adapter is, where to write
//! predictions) plus the in-memory ground-truth trajectories the
//! report joins against.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::ports::predictor_target::PredictorTarget;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidateTrainedRunRequest {
    predictor_target: PredictorTarget,
    ground_truth: Vec<TrainingTrajectory>,
}

impl ValidateTrainedRunRequest {
    pub fn new(predictor_target: PredictorTarget, ground_truth: Vec<TrainingTrajectory>) -> Self {
        Self {
            predictor_target,
            ground_truth,
        }
    }

    pub fn predictor_target(&self) -> &PredictorTarget {
        &self.predictor_target
    }

    pub fn ground_truth(&self) -> &[TrainingTrajectory] {
        &self.ground_truth
    }
}
