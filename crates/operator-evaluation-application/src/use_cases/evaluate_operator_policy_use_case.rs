//! `EvaluateOperatorPolicyUseCase` scores a set of `EvaluationPair`
//! values using an injected `ActionContractValidator`. The use case is
//! deliberately in-memory: it takes a slice of pairs and returns a
//! report. Wire/stream adapters (JSONL prediction readers, dashboard
//! writers) belong to a future infra crate.

use operator_evaluation_domain::outcome::prediction_evaluation_outcome::PredictionEvaluationOutcome;
use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
use operator_evaluation_domain::prediction::shape_violation_record::ShapeViolationRecord;
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
        shape_violations: &[ShapeViolationRecord],
    ) -> Result<EvaluationReport, EvaluateOperatorPolicyError> {
        let mut outcomes = Vec::with_capacity(pairs.len());
        for pair in pairs {
            let ground_truth = pair.ground_truth();
            let prediction = pair.prediction();
            let violations = match self.validator.validate(
                prediction.action(),
                ground_truth.about(),
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
        Ok(EvaluationReport::from_outcomes_and_shape_violations(
            outcomes,
            shape_violations.to_vec(),
        ))
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
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Inspect the expected memory node.",
            )
            .unwrap(),
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
        let report = use_case.execute(&pairs, &[]).unwrap();
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
        let report = use_case.execute(&pairs, &[]).unwrap();
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
        let report = use_case.execute(&pairs, &[]).unwrap();
        assert_eq!(report.contract_valid_count(), 0);
        assert_eq!(report.tool_match_count(), 1);
    }

    fn ask_trajectory(id: &str) -> TrainingTrajectory {
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
            AskArguments::new("ground truth question").unwrap(),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(id).unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("a:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.ask").unwrap(),
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Ask the prepared ground truth question.",
            )
            .unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
            action,
        )
        .unwrap()
    }

    fn stop_trajectory(id: &str) -> TrainingTrajectory {
        use operator_shared_domain::action::stop_action::StopAction;
        use operator_shared_domain::action::stop_reason::StopReason;

        let action =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(id).unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("a:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.stop").unwrap(),
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Stop after reaching the answer.",
            )
            .unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
            action,
        )
        .unwrap()
    }

    fn custom_pair(gt: TrainingTrajectory, predicted_action: OperatorAction) -> EvaluationPair {
        let prediction = PredictedAction::new(gt.id().clone(), predicted_action);
        EvaluationPair::new(gt, prediction).unwrap()
    }

    #[test]
    fn per_tool_bucketing_through_use_case_separates_inspect_ask_and_stop() {
        use operator_shared_domain::action::stop_action::StopAction;
        use operator_shared_domain::action::stop_reason::StopReason;
        use operator_shared_domain::tool::kernel_tool::KernelTool;

        let use_case =
            EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());

        // Inspect bucket: 2 pairs, one exact and one wrong-tool prediction.
        let inspect_exact = pair("t:i1", inspect("node:1"));
        let inspect_wrong = pair("t:i2", ask("predicted instead"));

        // Ask bucket: 1 pair, exact match.
        let ask_exact = custom_pair(ask_trajectory("t:a1"), ask("ground truth question"));

        // Stop bucket (None): 1 pair, exact; 1 pair with wrong prediction.
        let stop_exact = custom_pair(
            stop_trajectory("t:s1"),
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap()),
        );
        let stop_wrong = custom_pair(stop_trajectory("t:s2"), ask("predicted instead of stop"));

        let report = use_case
            .execute(
                &[
                    inspect_exact,
                    inspect_wrong,
                    ask_exact,
                    stop_exact,
                    stop_wrong,
                ],
                &[],
            )
            .unwrap();

        assert_eq!(report.total(), 5);

        let per_tool = report.per_tool();
        assert_eq!(per_tool.len(), 3);

        let inspect_bucket = per_tool
            .iter()
            .find(|m| m.tool() == Some(KernelTool::Inspect))
            .expect("inspect bucket exists");
        assert_eq!(inspect_bucket.total(), 2);
        assert_eq!(inspect_bucket.exact_matches(), 1);
        assert_eq!(inspect_bucket.tool_matches(), 1);

        let ask_bucket = per_tool
            .iter()
            .find(|m| m.tool() == Some(KernelTool::Ask))
            .expect("ask bucket exists");
        assert_eq!(ask_bucket.total(), 1);
        assert_eq!(ask_bucket.exact_matches(), 1);
        assert_eq!(ask_bucket.tool_matches(), 1);

        let stop_bucket = per_tool
            .iter()
            .find(|m| m.tool().is_none())
            .expect("stop/escalate bucket exists");
        assert_eq!(stop_bucket.total(), 2);
        assert_eq!(stop_bucket.exact_matches(), 1);
        assert_eq!(stop_bucket.tool_matches(), 1);
    }

    #[test]
    fn shape_violations_are_counted_in_report_denominator() {
        let use_case =
            EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());
        let pairs = vec![pair("t:1", inspect("node:1"))];
        let violations = vec![
            ShapeViolationRecord::new(7, Some(StepId::parse("s:bad").unwrap()), "bad action")
                .unwrap(),
        ];

        let report = use_case.execute(&pairs, &violations).unwrap();

        assert_eq!(report.parsed_count(), 1);
        assert_eq!(report.shape_invalid_count(), 1);
        assert_eq!(report.total(), 2);
        assert_eq!(report.exact_match_count(), 1);
        assert_eq!(report.contract_valid_count(), 1);
        assert!((report.exact_match_rate() - 0.5).abs() < f64::EPSILON);
        assert!((report.contract_validity_rate() - 0.5).abs() < f64::EPSILON);
    }
}
