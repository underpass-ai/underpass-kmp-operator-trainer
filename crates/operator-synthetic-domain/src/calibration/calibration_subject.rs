//! Model-facing subject for one teacher calibration decision.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationSubject {
    about: AboutId,
    mode: OperatorMode,
    task_family: TaskFamily,
    goal: TrajectoryGoal,
    allowed_tools: AllowedTools,
    visible_state: VisibleState,
}

impl CalibrationSubject {
    pub fn new(
        about: AboutId,
        mode: OperatorMode,
        task_family: TaskFamily,
        goal: TrajectoryGoal,
        allowed_tools: AllowedTools,
        visible_state: VisibleState,
    ) -> SyntheticDomainResult<Self> {
        if allowed_tools.mode() != mode {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "calibration_subject.allowed_tools.mode",
                expected: mode.as_str().to_string(),
                actual: allowed_tools.mode().as_str().to_string(),
            }
            .into());
        }
        Ok(Self {
            about,
            mode,
            task_family,
            goal,
            allowed_tools,
            visible_state,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
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
            )
            .is_err()
        );
    }
}
