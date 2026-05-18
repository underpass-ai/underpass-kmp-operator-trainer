//! Result of evaluating one prediction against its ground truth. Carries
//! the trajectory id, the action kinds on both sides, the contract
//! violations produced by the configured validator, and two derived
//! flags: exact-match and tool-match (same `ToolCall` tool, or both Stop,
//! or both Escalate).

use operator_shared_domain::action::operator_action::{OperatorAction, OperatorActionKind};
use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionEvaluationOutcome {
    trajectory_id: TrainingTrajectoryId,
    ground_truth_kind: OperatorActionKind,
    prediction_kind: OperatorActionKind,
    ground_truth_tool: Option<KernelTool>,
    contract_violations: ContractViolations,
    is_exact_match: bool,
    is_tool_match: bool,
}

impl PredictionEvaluationOutcome {
    pub fn evaluate(
        trajectory_id: TrainingTrajectoryId,
        ground_truth: &OperatorAction,
        prediction: &OperatorAction,
        contract_violations: ContractViolations,
    ) -> Self {
        Self {
            trajectory_id,
            ground_truth_kind: ground_truth.kind(),
            prediction_kind: prediction.kind(),
            ground_truth_tool: ground_truth.tool(),
            is_exact_match: ground_truth == prediction,
            is_tool_match: same_high_level_choice(ground_truth, prediction),
            contract_violations,
        }
    }

    pub fn trajectory_id(&self) -> &TrainingTrajectoryId {
        &self.trajectory_id
    }

    pub fn ground_truth_kind(&self) -> OperatorActionKind {
        self.ground_truth_kind
    }

    pub fn prediction_kind(&self) -> OperatorActionKind {
        self.prediction_kind
    }

    pub fn ground_truth_tool(&self) -> Option<KernelTool> {
        self.ground_truth_tool
    }

    pub fn contract_violations(&self) -> &ContractViolations {
        &self.contract_violations
    }

    pub fn is_exact_match(&self) -> bool {
        self.is_exact_match
    }

    pub fn is_tool_match(&self) -> bool {
        self.is_tool_match
    }

    pub fn is_contract_valid(&self) -> bool {
        self.contract_violations.is_empty()
    }
}

fn same_high_level_choice(ground_truth: &OperatorAction, prediction: &OperatorAction) -> bool {
    match (ground_truth, prediction) {
        (OperatorAction::ToolCall(gt), OperatorAction::ToolCall(pr)) => gt.tool() == pr.tool(),
        (OperatorAction::Stop(_), OperatorAction::Stop(_))
        | (OperatorAction::Escalate(_), OperatorAction::Escalate(_)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;

    fn inspect(memory: &str) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse(memory).unwrap()),
        )))
    }

    fn ask(query: &str) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
            AskArguments::new(query).unwrap(),
        )))
    }

    fn stop() -> OperatorAction {
        OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
    }

    fn id() -> TrainingTrajectoryId {
        TrainingTrajectoryId::parse("t:1").unwrap()
    }

    #[test]
    fn exact_match_implies_tool_match() {
        let gt = inspect("node:1");
        let pred = inspect("node:1");
        let outcome =
            PredictionEvaluationOutcome::evaluate(id(), &gt, &pred, ContractViolations::new());
        assert!(outcome.is_exact_match());
        assert!(outcome.is_tool_match());
    }

    #[test]
    fn same_tool_with_different_arguments_is_tool_match_only() {
        let gt = inspect("node:1");
        let pred = inspect("node:2");
        let outcome =
            PredictionEvaluationOutcome::evaluate(id(), &gt, &pred, ContractViolations::new());
        assert!(!outcome.is_exact_match());
        assert!(outcome.is_tool_match());
    }

    #[test]
    fn different_tools_are_neither_exact_nor_tool_match() {
        let gt = inspect("node:1");
        let pred = ask("why");
        let outcome =
            PredictionEvaluationOutcome::evaluate(id(), &gt, &pred, ContractViolations::new());
        assert!(!outcome.is_exact_match());
        assert!(!outcome.is_tool_match());
    }

    #[test]
    fn stop_versus_tool_call_does_not_match() {
        let outcome = PredictionEvaluationOutcome::evaluate(
            id(),
            &inspect("node:1"),
            &stop(),
            ContractViolations::new(),
        );
        assert!(!outcome.is_tool_match());
        assert_eq!(outcome.ground_truth_kind(), OperatorActionKind::ToolCall);
        assert_eq!(outcome.prediction_kind(), OperatorActionKind::Stop);
    }

    #[test]
    fn contract_violations_propagate() {
        let mut violations = ContractViolations::new();
        violations.push(operator_shared_domain::contract::contract_violation::ContractViolation::new(
            operator_shared_domain::contract::contract_violation_code::ContractViolationCode::BudgetExhausted,
            "x",
            "y",
        ));
        let outcome = PredictionEvaluationOutcome::evaluate(id(), &stop(), &stop(), violations);
        assert!(!outcome.is_contract_valid());
        assert_eq!(outcome.contract_violations().len(), 1);
    }
}
