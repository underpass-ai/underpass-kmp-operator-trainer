//! A `PredictedAction` is the action a model produced for a specific
//! trajectory's input context. It carries the trajectory id so the
//! evaluation use case can join predictions with their ground truths
//! safely.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedAction {
    trajectory_id: TrainingTrajectoryId,
    action: OperatorAction,
}

impl PredictedAction {
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

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;

    #[test]
    fn exposes_id_and_action() {
        let id = TrainingTrajectoryId::parse("t:1").unwrap();
        let action =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        let predicted = PredictedAction::new(id.clone(), action.clone());
        assert_eq!(predicted.trajectory_id(), &id);
        assert_eq!(predicted.action(), &action);
    }
}
