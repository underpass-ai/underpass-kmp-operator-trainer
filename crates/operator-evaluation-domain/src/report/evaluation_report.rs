//! Aggregate evaluation outcome: every per-prediction record plus
//! derived rates and per-`KernelTool` metrics.

use std::collections::BTreeMap;

use operator_shared_domain::tool::kernel_tool::KernelTool;

use crate::outcome::prediction_evaluation_outcome::PredictionEvaluationOutcome;
use crate::prediction::shape_violation_record::ShapeViolationRecord;
use crate::report::tool_evaluation_metric::ToolEvaluationMetric;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationReport {
    outcomes: Vec<PredictionEvaluationOutcome>,
    shape_violations: Vec<ShapeViolationRecord>,
    per_tool: Vec<ToolEvaluationMetric>,
}

impl EvaluationReport {
    pub fn from_outcomes(outcomes: Vec<PredictionEvaluationOutcome>) -> Self {
        Self::from_outcomes_and_shape_violations(outcomes, Vec::new())
    }

    pub fn from_outcomes_and_shape_violations(
        outcomes: Vec<PredictionEvaluationOutcome>,
        shape_violations: Vec<ShapeViolationRecord>,
    ) -> Self {
        let per_tool = aggregate_per_tool(&outcomes);
        Self {
            outcomes,
            shape_violations,
            per_tool,
        }
    }

    pub fn outcomes(&self) -> &[PredictionEvaluationOutcome] {
        &self.outcomes
    }

    pub fn shape_violations(&self) -> &[ShapeViolationRecord] {
        &self.shape_violations
    }

    pub fn per_tool(&self) -> &[ToolEvaluationMetric] {
        &self.per_tool
    }

    pub fn total(&self) -> usize {
        self.parsed_count() + self.shape_invalid_count()
    }

    pub fn parsed_count(&self) -> usize {
        self.outcomes.len()
    }

    pub fn shape_invalid_count(&self) -> usize {
        self.shape_violations.len()
    }

    pub fn exact_match_count(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_exact_match()).count()
    }

    pub fn tool_match_count(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_tool_match()).count()
    }

    pub fn contract_valid_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.is_contract_valid())
            .count()
    }

    /// Number of stop ground-truths whose prediction is a stop with the same
    /// reason (evidence subset ignored). See
    /// `PredictionEvaluationOutcome::is_stop_decision_match`.
    pub fn stop_decision_match_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.is_stop_decision_match())
            .count()
    }

    /// Number of ground-truths that are stop actions (denominator for the
    /// stop-decision-match rate).
    pub fn stop_ground_truth_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.ground_truth_is_stop())
            .count()
    }

    /// Stop-decision match rate: of the stop ground-truths, the fraction the
    /// model also stopped on for the same reason. A more faithful stop metric
    /// than exact-match, which over-penalizes a valid-but-different evidence
    /// subset. Returns 0.0 when there are no stop ground-truths.
    pub fn stop_decision_match_rate(&self) -> f64 {
        rate(
            self.stop_decision_match_count(),
            self.stop_ground_truth_count(),
        )
    }

    pub fn exact_match_rate(&self) -> f64 {
        rate(self.exact_match_count(), self.total())
    }

    pub fn tool_match_rate(&self) -> f64 {
        rate(self.tool_match_count(), self.total())
    }

    pub fn contract_validity_rate(&self) -> f64 {
        rate(self.contract_valid_count(), self.total())
    }

    pub fn shape_invalid_rate(&self) -> f64 {
        rate(self.shape_invalid_count(), self.total())
    }
}

#[allow(clippy::cast_precision_loss)]
fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn aggregate_per_tool(outcomes: &[PredictionEvaluationOutcome]) -> Vec<ToolEvaluationMetric> {
    let mut buckets: BTreeMap<Option<KernelTool>, ToolEvaluationMetric> = BTreeMap::new();
    for outcome in outcomes {
        let key = outcome.ground_truth_tool();
        let entry = buckets
            .entry(key)
            .or_insert_with(|| ToolEvaluationMetric::empty_for(key));
        entry.record(
            outcome.is_exact_match(),
            outcome.is_tool_match(),
            outcome.is_contract_valid(),
        );
    }
    buckets.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::contract::contract_violations::ContractViolations;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
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

    fn stop_with(reason: StopReason, evidence: &str) -> OperatorAction {
        OperatorAction::Stop(
            StopAction::new(reason, None, vec![MemoryRef::parse(evidence).unwrap()]).unwrap(),
        )
    }

    fn outcome(
        trajectory: &str,
        gt: &OperatorAction,
        pred: &OperatorAction,
    ) -> PredictionEvaluationOutcome {
        PredictionEvaluationOutcome::evaluate(
            TrainingTrajectoryId::parse(trajectory).unwrap(),
            gt,
            pred,
            ContractViolations::new(),
        )
    }

    #[test]
    fn aggregates_total_and_rates() {
        let report = EvaluationReport::from_outcomes(vec![
            outcome("t:1", &inspect("node:1"), &inspect("node:1")), // exact
            outcome("t:2", &inspect("node:1"), &inspect("node:2")), // tool only
            outcome("t:3", &inspect("node:1"), &ask("why")),        // miss
        ]);
        assert_eq!(report.total(), 3);
        assert_eq!(report.exact_match_count(), 1);
        assert_eq!(report.tool_match_count(), 2);
        assert_eq!(report.contract_valid_count(), 3);
        assert!((report.exact_match_rate() - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stop_decision_match_rate_ignores_evidence_subset() {
        let report = EvaluationReport::from_outcomes(vec![
            // stop gt, right reason, DIFFERENT evidence -> decision match, not exact
            outcome(
                "t:1",
                &stop_with(StopReason::AnswerReady, "node:a"),
                &stop_with(StopReason::AnswerReady, "node:b"),
            ),
            // stop gt, WRONG reason -> not a decision match
            outcome(
                "t:2",
                &stop_with(StopReason::AnswerReady, "node:a"),
                &stop_with(StopReason::BudgetExhausted, "node:a"),
            ),
            // non-stop gt -> not counted in the stop denominator
            outcome("t:3", &inspect("node:1"), &inspect("node:1")),
        ]);
        assert_eq!(report.stop_ground_truth_count(), 2);
        assert_eq!(report.stop_decision_match_count(), 1);
        assert!((report.stop_decision_match_rate() - 0.5).abs() < f64::EPSILON);
        // exact-match credits none of the stops (evidence differs / reason differs)
        assert_eq!(report.exact_match_count(), 1); // only the inspect
    }

    #[test]
    fn shape_violations_count_as_invalid_rows() {
        let violation = ShapeViolationRecord::new(7, None, "bad action").unwrap();
        let report = EvaluationReport::from_outcomes_and_shape_violations(
            vec![
                outcome("t:1", &inspect("node:1"), &inspect("node:1")),
                outcome("t:2", &inspect("node:1"), &ask("why")),
            ],
            vec![violation.clone()],
        );

        assert_eq!(report.parsed_count(), 2);
        assert_eq!(report.shape_invalid_count(), 1);
        assert_eq!(report.shape_violations(), &[violation]);
        assert_eq!(report.total(), 3);
        assert_eq!(report.exact_match_count(), 1);
        assert_eq!(report.tool_match_count(), 1);
        assert_eq!(report.contract_valid_count(), 2);
        assert!((report.exact_match_rate() - 1.0 / 3.0).abs() < f64::EPSILON);
        assert!((report.tool_match_rate() - 1.0 / 3.0).abs() < f64::EPSILON);
        assert!((report.contract_validity_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
        assert!((report.shape_invalid_rate() - 1.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn per_tool_bucketing_includes_stop() {
        let report = EvaluationReport::from_outcomes(vec![
            outcome("t:1", &inspect("node:1"), &inspect("node:1")),
            outcome("t:2", &stop(), &stop()),
            outcome("t:3", &stop(), &inspect("node:2")),
        ]);
        let per_tool = report.per_tool();
        assert_eq!(per_tool.len(), 2); // Inspect bucket + Stop bucket (None)
        let stop_bucket = per_tool
            .iter()
            .find(|m| m.tool().is_none())
            .expect("stop bucket exists");
        assert_eq!(stop_bucket.total(), 2);
        assert_eq!(stop_bucket.exact_matches(), 1);
    }
}
