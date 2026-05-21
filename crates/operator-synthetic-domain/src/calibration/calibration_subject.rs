//! Model-facing subject for one teacher calibration decision.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::calibration::prepared_operator_action::PreparedOperatorAction;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationSubject {
    about: AboutId,
    mode: OperatorMode,
    task_family: TaskFamily,
    goal: TrajectoryGoal,
    allowed_tools: AllowedTools,
    visible_state: VisibleState,
    prepared_action: Option<PreparedOperatorAction>,
}

impl CalibrationSubject {
    pub fn new(
        about: AboutId,
        mode: OperatorMode,
        task_family: TaskFamily,
        goal: TrajectoryGoal,
        allowed_tools: AllowedTools,
        visible_state: VisibleState,
        prepared_action: Option<PreparedOperatorAction>,
    ) -> SyntheticDomainResult<Self> {
        if allowed_tools.mode() != mode {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "calibration_subject.allowed_tools.mode",
                expected: mode.as_str().to_string(),
                actual: allowed_tools.mode().as_str().to_string(),
            }
            .into());
        }
        if let Some(action) = &prepared_action {
            let tool = action.tool();
            if !allowed_tools.contains(tool) {
                return Err(SyntheticDomainError::PreparedActionToolOutsideMode {
                    mode: mode.as_str().to_string(),
                    tool: tool.as_str().to_string(),
                });
            }
        }
        Ok(Self {
            about,
            mode,
            task_family,
            goal,
            allowed_tools,
            visible_state,
            prepared_action,
        })
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn mode(&self) -> OperatorMode {
        self.mode
    }

    pub fn task_family(&self) -> &TaskFamily {
        &self.task_family
    }

    pub fn goal(&self) -> &TrajectoryGoal {
        &self.goal
    }

    pub fn allowed_tools(&self) -> &AllowedTools {
        &self.allowed_tools
    }

    pub fn visible_state(&self) -> &VisibleState {
        &self.visible_state
    }

    pub fn prepared_action(&self) -> Option<&PreparedOperatorAction> {
        self.prepared_action.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    #[test]
    fn builds_when_allowed_tools_match_mode() {
        let subject = CalibrationSubject::new(
            AboutId::parse("about:test").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            TrajectoryGoal::parse("Inspect the visible node.").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
            None,
        )
        .unwrap();
        assert_eq!(subject.mode(), OperatorMode::Read);
    }

    #[test]
    fn rejects_allowed_tools_for_different_mode() {
        assert!(
            CalibrationSubject::new(
                AboutId::parse("about:test").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.inspect").unwrap(),
                TrajectoryGoal::parse("Inspect the visible node.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Write),
                VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_prepared_action_outside_mode() {
        let prepared = PreparedOperatorAction::new(OperatorAction::ToolCall(ToolCallAction::new(
            ToolArguments::WriteMemory(
                WriteMemoryArguments::new("summary", "body", vec![]).unwrap(),
            ),
        )))
        .unwrap();

        assert!(
            CalibrationSubject::new(
                AboutId::parse("about:test").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.inspect").unwrap(),
                TrajectoryGoal::parse("Inspect the visible node.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
                Some(prepared),
            )
            .is_err()
        );
    }
}
