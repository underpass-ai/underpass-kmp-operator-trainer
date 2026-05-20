//! `TrainingTrajectory` is the aggregate root of the shared bounded
//! context. Its named constructor refuses to build an invalid trajectory:
//!
//! - `allowed_tools.mode()` must equal `mode`,
//! - `allowed_tools` must equal `AllowedTools::for_mode(mode)`,
//! - if the target action is a `ToolCall`, its tool must be in
//!   `allowed_tools`.
//!
//! The constructor does **not** run the full action contract validator;
//! that is the job of `ValidateTrajectoryUseCase` in the application
//! layer. Constructor invariants are the minimum that makes the
//! trajectory representable.

use crate::action::operator_action::OperatorAction;
use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::ids::about_id::AboutId;
use crate::ids::step_id::StepId;
use crate::ids::training_trajectory_id::TrainingTrajectoryId;
use crate::mode::allowed_tools::AllowedTools;
use crate::mode::operator_mode::OperatorMode;
use crate::value_objects::task_family::TaskFamily;
use crate::value_objects::trajectory_goal::TrajectoryGoal;
use crate::visible_state::visible_state::VisibleState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingTrajectory {
    id: TrainingTrajectoryId,
    step_id: StepId,
    about: AboutId,
    mode: OperatorMode,
    task_family: TaskFamily,
    goal: TrajectoryGoal,
    allowed_tools: AllowedTools,
    visible_state: VisibleState,
    target_action: OperatorAction,
}

impl TrainingTrajectory {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TrainingTrajectoryId,
        step_id: StepId,
        about: AboutId,
        mode: OperatorMode,
        task_family: TaskFamily,
        goal: TrajectoryGoal,
        allowed_tools: AllowedTools,
        visible_state: VisibleState,
        target_action: OperatorAction,
    ) -> DomainResult<Self> {
        if allowed_tools.mode() != mode {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "allowed_tools.mode",
                expected: mode.as_str().to_string(),
                actual: allowed_tools.mode().as_str().to_string(),
            });
        }
        let canonical = AllowedTools::for_mode(mode);
        if canonical.as_slice() != allowed_tools.as_slice() {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "allowed_tools",
                expected: tool_list(canonical.as_slice()),
                actual: tool_list(allowed_tools.as_slice()),
            });
        }
        if let Some(tool) = target_action.tool()
            && !allowed_tools.contains(tool)
        {
            return Err(DomainError::ToolOutsideMode {
                mode: mode.as_str(),
                tool: tool.as_str(),
            });
        }
        Ok(Self {
            id,
            step_id,
            about,
            mode,
            task_family,
            goal,
            allowed_tools,
            visible_state,
            target_action,
        })
    }

    pub fn id(&self) -> &TrainingTrajectoryId {
        &self.id
    }

    pub fn step_id(&self) -> &StepId {
        &self.step_id
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

    pub fn target_action(&self) -> &OperatorAction {
        &self.target_action
    }
}

fn tool_list(tools: &[crate::tool::kernel_tool::KernelTool]) -> String {
    tools
        .iter()
        .copied()
        .map(crate::tool::kernel_tool::KernelTool::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::stop_action::StopAction;
    use crate::action::stop_reason::StopReason;
    use crate::action::tool_call_action::ToolCallAction;
    use crate::tool::kernel_tool::KernelTool;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::tool_arguments::ToolArguments;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::visible_state::visible_state_builder::VisibleStateBuilder;

    fn read_inspect_trajectory() -> DomainResult<TrainingTrajectory> {
        let target = MemoryRef::parse("node:1")?;
        let visible = VisibleStateBuilder::new()
            .with_known_ref(target.clone())
            .build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse("traj:1")?,
            StepId::parse("step:1")?,
            AboutId::parse("about:1")?,
            OperatorMode::Read,
            TaskFamily::parse("read.inspect")?,
            TrajectoryGoal::parse("Inspect the visible memory node.")?,
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
    }

    #[test]
    fn builds_canonical_trajectory() {
        let trajectory = read_inspect_trajectory().expect("inspect trajectory must build");
        assert_eq!(trajectory.mode(), OperatorMode::Read);
        assert_eq!(trajectory.target_action().tool(), Some(KernelTool::Inspect));
    }

    #[test]
    fn refuses_action_outside_allowed_tools() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::Stop(
            StopAction::new(StopReason::AnswerReady, None, vec![])
                .expect("stop without answer is valid"),
        );
        // Read mode but allowed_tools claims to be for Write — should fail.
        let mismatch = TrainingTrajectory::new(
            TrainingTrajectoryId::parse("traj:1").unwrap(),
            StepId::parse("step:1").unwrap(),
            AboutId::parse("about:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.stop").unwrap(),
            TrajectoryGoal::parse("Stop with the visible evidence.").unwrap(),
            AllowedTools::for_mode(OperatorMode::Write),
            visible,
            action,
        )
        .expect_err("allowed_tools.mode mismatch must be rejected");
        assert!(matches!(
            mismatch,
            DomainError::TrajectoryInvariantMismatch {
                field: "allowed_tools.mode",
                ..
            }
        ));
    }

    #[test]
    fn refuses_tool_outside_mode() {
        let target = MemoryRef::parse("node:1").unwrap();
        let visible = VisibleStateBuilder::new()
            .with_known_ref(target.clone())
            .build();
        let write_action =
            OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
                crate::tool_arguments::write_memory_arguments::WriteMemoryArguments::new(
                    "summary",
                    "body",
                    vec![],
                )
                .unwrap(),
            )));
        let result = TrainingTrajectory::new(
            TrainingTrajectoryId::parse("traj:1").unwrap(),
            StepId::parse("step:1").unwrap(),
            AboutId::parse("about:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.write_attempt").unwrap(),
            TrajectoryGoal::parse("Reject write tools while operating in read mode.").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            write_action,
        );
        assert!(matches!(result, Err(DomainError::ToolOutsideMode { .. })));
    }
}
