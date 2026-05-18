//! `EvaluateOperatorPolicyUseCase` scores a set of `EvaluationPair`
//! values using an injected `ActionContractValidator`. The use case is
//! deliberately in-memory: it takes a slice of pairs and returns a
//! report. Wire/stream adapters (JSONL prediction readers, dashboard
//! writers) belong to a future infra crate.

use operator_evaluation_domain::outcome::prediction_evaluation_outcome::PredictionEvaluationOutcome;
use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;

use crate::error::evaluate_operator_policy_error::EvaluateOperatorPolicyError;

#[derive(Debug)]
pub struct EvaluateOperatorPolicyUseCase<V: ActionContractValidator> {
    validator: V,
}

impl<V: ActionContractValidator> EvaluateOperatorPolicyUseCase<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }

    pub fn execute(
        &self,
        pairs: &[EvaluationPair],
    ) -> Result<EvaluationReport, EvaluateOperatorPolicyError> {
        let mut outcomes = Vec::with_capacity(pairs.len());
        for pair in pairs {
            let ground_truth = pair.ground_truth();
            let prediction = pair.prediction();
            let violations = match self.validator.validate(
                prediction.action(),
                ground_truth.mode(),
                ground_truth.visible_state(),
            ) {
                Ok(()) => {
                    operator_shared_domain::contract::contract_violations::ContractViolations::new()
                }
                Err(violations) => violations,
            };
            outcomes.push(PredictionEvaluationOutcome::evaluate(
                ground_truth.id().clone(),
                ground_truth.target_action(),
                prediction.action(),
                violations,
            ));
        }
        Ok(EvaluationReport::from_outcomes(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_evaluation_domain::prediction::predicted_action::PredictedAction;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    fn inspect_trajectory(id: &str, memory: &str) -> TrainingTrajectory {
        let target = MemoryRef::parse(memory).unwrap();
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

    fn pair(id: &str, predicted_action: OperatorAction) -> EvaluationPair {
        let gt = inspect_trajectory(id, "node:1");
        let prediction =
            PredictedAction::new(TrainingTrajectoryId::parse(id).unwrap(), predicted_action);
        EvaluationPair::new(gt, prediction).unwrap()
    }

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

    #[test]
    fn perfect_predictions_yield_100_percent_match() {
        let use_case =
            EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());
        let pairs = vec![
            pair("t:1", inspect("node:1")),
            pair("t:2", inspect("node:1")),
        ];
        let report = use_case.execute(&pairs).unwrap();
        assert_eq!(report.total(), 2);
        assert_eq!(report.exact_match_count(), 2);
        assert_eq!(report.tool_match_count(), 2);
        assert_eq!(report.contract_valid_count(), 2);
        assert!((report.exact_match_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adversarial_predictions_yield_zero_match() {
        let use_case =
            EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());
        let pairs = vec![pair("t:1", ask("nothing useful"))];
        let report = use_case.execute(&pairs).unwrap();
        assert_eq!(report.exact_match_count(), 0);
        assert_eq!(report.tool_match_count(), 0);
        // The prediction is Ask, which is allowed in Read mode and does
        // not reference any visible state — so it is contract-valid even
        // though it doesn't match the ground truth.
        assert_eq!(report.contract_valid_count(), 1);
    }

    #[test]
    fn predictions_referencing_unknown_refs_violate_contract() {
        let use_case =
            EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());
        let pairs = vec![pair("t:1", inspect("node:absent"))];
        let report = use_case.execute(&pairs).unwrap();
        assert_eq!(report.contract_valid_count(), 0);
        assert_eq!(report.tool_match_count(), 1);
    }
}
