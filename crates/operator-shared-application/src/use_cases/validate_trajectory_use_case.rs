//! `ValidateTrajectoryUseCase` runs an `ActionContractValidator` over a
//! single, already-constructed `TrainingTrajectory`. The trajectory's
//! constructor invariants are guaranteed by the type; this use case
//! validates the *contract* invariants that depend on `VisibleState`
//! contents and budget.

use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::use_cases::validate_trajectory_error::ValidateTrajectoryError;

#[derive(Debug)]
pub struct ValidateTrajectoryUseCase<V: ActionContractValidator> {
    validator: V,
}

impl<V: ActionContractValidator> ValidateTrajectoryUseCase<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }

    pub fn execute(&self, trajectory: &TrainingTrajectory) -> Result<(), ValidateTrajectoryError> {
        self.validator
            .validate(
                trajectory.target_action(),
                trajectory.about(),
                trajectory.mode(),
                trajectory.visible_state(),
            )
            .map_err(ValidateTrajectoryError::from_violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
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

    #[test]
    fn accepts_consistent_inspect_trajectory() {
        let target = MemoryRef::parse("node:1").unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        let trajectory = TrainingTrajectory::new(
            TrainingTrajectoryId::parse("t:1").unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("a:1").unwrap(),
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
        .unwrap();

        let validator = CompositeActionContractValidator::default_strict();
        let use_case = ValidateTrajectoryUseCase::new(validator);
        use_case
            .execute(&trajectory)
            .expect("trajectory is contract-valid");
    }

    #[test]
    fn rejects_inspect_of_unknown_ref() {
        // visible state knows nothing; action inspects an unknown ref.
        let target = MemoryRef::parse("node:absent").unwrap();
        let visible = VisibleState::assemble([], [], None, BudgetSnapshot::unbounded());
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        let trajectory = TrainingTrajectory::new(
            TrainingTrajectoryId::parse("t:1").unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("a:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Inspect the unknown memory node.",
            )
            .unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
        .unwrap();

        let validator = CompositeActionContractValidator::default_strict();
        let use_case = ValidateTrajectoryUseCase::new(validator);
        let err = use_case
            .execute(&trajectory)
            .expect_err("inspect of unknown ref must fail");
        assert!(matches!(
            err,
            ValidateTrajectoryError::ContractViolations { .. }
        ));
    }
}
