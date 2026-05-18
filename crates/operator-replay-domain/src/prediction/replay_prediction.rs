//! Input to the replay use case: a trajectory id plus the action the
//! caller wants to execute against KMP. Independent of evaluation's
//! `PredictedAction` because replay and evaluation are independent
//! bounded contexts; identical structure is acceptable per ADR 0008.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayPrediction {
    trajectory_id: TrainingTrajectoryId,
    action: OperatorAction,
}

impl ReplayPrediction {
    pub fn new(trajectory_id: TrainingTrajectoryId, action: OperatorAction) -> Self {
        Self {
            trajectory_id,
            action,
        }
    }

    pub fn trajectory_id(&self) -> &TrainingTrajectoryId {
        &self.trajectory_id
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }
}
