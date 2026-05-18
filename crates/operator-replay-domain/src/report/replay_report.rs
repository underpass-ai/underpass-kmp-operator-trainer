//! Aggregate replay outcome: every per-prediction execution plus
//! derived counts and rates. Refuses to be constructed empty.

use crate::error::replay_domain_error::ReplayDomainError;
use crate::error::replay_domain_result::ReplayDomainResult;
use crate::execution::replay_execution::ReplayExecution;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReport {
    executions: Vec<ReplayExecution>,
}

impl ReplayReport {
    pub fn new(executions: Vec<ReplayExecution>) -> ReplayDomainResult<Self> {
        if executions.is_empty() {
            return Err(ReplayDomainError::EmptyReport);
        }
        Ok(Self { executions })
    }

    pub fn executions(&self) -> &[ReplayExecution] {
        &self.executions
    }

    pub fn total(&self) -> usize {
        self.executions.len()
    }

    pub fn successful_tool_calls(&self) -> usize {
        self.executions
            .iter()
            .filter(|e| e.outcome().is_tool_call_success())
            .count()
    }

    pub fn failed_tool_calls(&self) -> usize {
        self.executions
            .iter()
            .filter(|e| e.outcome().is_tool_call_failure())
            .count()
    }

    pub fn stop_or_escalate(&self) -> usize {
        self.executions
            .iter()
            .filter(|e| e.outcome().is_no_tool_call())
            .count()
    }

    /// Fraction of attempted tool calls that succeeded.
    ///
    /// Returns `None` when no tool call was attempted (every prediction
    /// was `Stop` or `Escalate`). The previous `f64` API returned `0.0`
    /// in that case, which was indistinguishable from "every attempt
    /// failed"; callers now match on `Option` and decide the display
    /// convention explicitly.
    #[allow(clippy::cast_precision_loss)]
    pub fn tool_call_success_rate(&self) -> Option<f64> {
        let attempted = self.successful_tool_calls() + self.failed_tool_calls();
        if attempted == 0 {
            None
        } else {
            Some(self.successful_tool_calls() as f64 / attempted as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::tool::kernel_tool::KernelTool;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
    use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

    use crate::outcome::replay_failure_reason::ReplayFailureReason;
    use crate::outcome::replay_outcome::ReplayOutcome;

    fn id(s: &str) -> TrainingTrajectoryId {
        TrainingTrajectoryId::parse(s).unwrap()
    }

    fn inspect_action() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )))
    }

    fn stop_action() -> OperatorAction {
        OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
    }

    fn inspect_outcome() -> ToolOutcome {
        ToolOutcome::Inspect(InspectOutcome::new(
            NonEmptyString::parse("s", "ctx").unwrap(),
            MemoryRef::parse("node:1").unwrap(),
            NonEmptyString::parse("claim", "ctx").unwrap(),
            vec![],
            vec![],
        ))
    }

    #[test]
    fn refuses_empty() {
        assert!(matches!(
            ReplayReport::new(vec![]),
            Err(ReplayDomainError::EmptyReport)
        ));
    }

    #[test]
    fn counts_and_rates_aggregate_correctly() {
        let executions = vec![
            ReplayExecution::new(
                id("t:1"),
                inspect_action(),
                ReplayOutcome::ToolSucceeded(inspect_outcome()),
            ),
            ReplayExecution::new(
                id("t:2"),
                inspect_action(),
                ReplayOutcome::ToolCallFailed {
                    tool: KernelTool::Inspect,
                    reason: ReplayFailureReason::Transport,
                },
            ),
            ReplayExecution::new(id("t:3"), stop_action(), ReplayOutcome::NoToolCall),
        ];
        let report = ReplayReport::new(executions).unwrap();
        assert_eq!(report.total(), 3);
        assert_eq!(report.successful_tool_calls(), 1);
        assert_eq!(report.failed_tool_calls(), 1);
        assert_eq!(report.stop_or_escalate(), 1);
        assert!((report.tool_call_success_rate().unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn success_rate_is_none_when_no_tool_call_attempted() {
        let executions = vec![ReplayExecution::new(
            id("t:1"),
            stop_action(),
            ReplayOutcome::NoToolCall,
        )];
        let report = ReplayReport::new(executions).unwrap();
        assert!(report.tool_call_success_rate().is_none());
    }
}
