//! `EvaluationPair` couples a ground-truth `TrainingTrajectory` with the
//! action a model predicted for the same trajectory id. Its named
//! constructor refuses to build a pair whose ids do not agree.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::error::evaluation_domain_error::EvaluationDomainError;
use crate::error::evaluation_domain_result::EvaluationDomainResult;
use crate::prediction::predicted_action::PredictedAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationPair {
    ground_truth: TrainingTrajectory,
    prediction: PredictedAction,
}

impl EvaluationPair {
    pub fn new(
        ground_truth: TrainingTrajectory,
        prediction: PredictedAction,
    ) -> EvaluationDomainResult<Self> {
        if prediction.trajectory_id() != ground_truth.id() {
            return Err(EvaluationDomainError::TrajectoryIdMismatch {
                expected: ground_truth.id().as_str().to_string(),
                actual: prediction.trajectory_id().as_str().to_string(),
            });
        }
        Ok(Self {
            ground_truth,
            prediction,
        })
    }

    pub fn ground_truth(&self) -> &TrainingTrajectory {
        &self.ground_truth
    }

    pub fn prediction(&self) -> &PredictedAction {
        &self.prediction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    fn inspect_trajectory(id: &str) -> TrainingTrajectory {
        let target = MemoryRef::parse("node:1").unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(id).unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("a:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
        .unwrap()
    }

    #[test]
    fn refuses_mismatched_trajectory_ids() {
        let gt = inspect_trajectory("t:gt");
        let predicted_action =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        let prediction = PredictedAction::new(
            TrainingTrajectoryId::parse("t:wrong").unwrap(),
            predicted_action,
        );
        let err = EvaluationPair::new(gt, prediction).unwrap_err();
        assert!(matches!(
            err,
            EvaluationDomainError::TrajectoryIdMismatch { ref expected, ref actual }
                if expected == "t:gt" && actual == "t:wrong"
        ));
    }

    #[test]
    fn builds_when_ids_match() {
        let gt = inspect_trajectory("t:gt");
        let prediction = PredictedAction::new(
            gt.id().clone(),
            OperatorAction::Stop(StopAction::new(StopReason::NoCandidate, None, vec![]).unwrap()),
        );
        let pair = EvaluationPair::new(gt, prediction).unwrap();
        assert_eq!(pair.ground_truth().id().as_str(), "t:gt");
    }
}
