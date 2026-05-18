//! A single replay step: the trajectory the prediction belongs to,
//! the predicted action, and the resulting `ReplayOutcome`. Constructed
//! by the use case, consumed by the aggregator report.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;

use crate::outcome::replay_outcome::ReplayOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExecution {
    trajectory_id: TrainingTrajectoryId,
    predicted_action: OperatorAction,
    outcome: ReplayOutcome,
}

impl ReplayExecution {
    pub fn new(
        trajectory_id: TrainingTrajectoryId,
        predicted_action: OperatorAction,
        outcome: ReplayOutcome,
    ) -> Self {
        Self {
            trajectory_id,
            predicted_action,
            outcome,
        }
    }

    pub fn trajectory_id(&self) -> &TrainingTrajectoryId {
        &self.trajectory_id
    }

    pub fn predicted_action(&self) -> &OperatorAction {
        &self.predicted_action
    }

    pub fn outcome(&self) -> &ReplayOutcome {
        &self.outcome
    }
}
