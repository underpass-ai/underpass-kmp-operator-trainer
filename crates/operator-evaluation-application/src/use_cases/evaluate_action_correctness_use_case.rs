use std::collections::BTreeMap;

use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
use operator_evaluation_domain::prediction::shape_violation_record::ShapeViolationRecord;
use operator_evaluation_domain::report::action_correctness_report::ActionCorrectnessReport;
use operator_evaluation_domain::report::field_stats::FieldStats;
use operator_evaluation_domain::report::tool_correctness_stats::ToolCorrectnessStats;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::correctness::action_correctness::ActionCorrectness;
use operator_shared_domain::contract::correctness::field_path::FieldPath;
use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Default)]
pub struct EvaluateActionCorrectnessUseCase;

impl EvaluateActionCorrectnessUseCase {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        pairs: &[EvaluationPair],
        shape_violations: &[ShapeViolationRecord],
    ) -> ActionCorrectnessReport {
        let mut builder = ActionCorrectnessReportBuilder::new();
        for pair in pairs {
            let ground_truth_action = pair.ground_truth().target_action();
            let predicted_action = pair.prediction().action();
            let outcome = predicted_action.evaluate_correctness(ground_truth_action);
            let tool_selection_correct =
                same_high_level_choice(ground_truth_action, predicted_action);
            let failed_fields = outcome
                .failed_fields()
                .map(|field| field.field_path().clone())
                .collect::<Vec<_>>();
            let field_results = outcome
                .field_results()
                .iter()
                .map(|field| (field.field_path().clone(), field.is_correct()))
                .collect::<Vec<_>>();
            builder.record_pair(
                ground_truth_action.tool(),
                outcome.is_correct(),
                tool_selection_correct,
                &failed_fields,
                field_results,
            );
        }
        builder.record_shape_invalid(shape_violations.len());
        builder.build()
    }
}

#[derive(Debug, Default)]
struct ActionCorrectnessReportBuilder {
    total: usize,
    action_correct_count: usize,
    tool_selection_correct_count: usize,
    shape_invalid_count: usize,
    per_field_correctness: BTreeMap<FieldPath, FieldStats>,
    per_tool: BTreeMap<Option<KernelTool>, ToolCorrectnessStats>,
}

impl ActionCorrectnessReportBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn record_pair(
        &mut self,
        ground_truth_tool: Option<KernelTool>,
        action_correct: bool,
        tool_selection_correct: bool,
        failed_fields: &[FieldPath],
        field_results: Vec<(FieldPath, bool)>,
    ) {
        self.total += 1;
        if action_correct {
            self.action_correct_count += 1;
        }
        if tool_selection_correct {
            self.tool_selection_correct_count += 1;
        }
        for (field_path, is_correct) in field_results {
            self.per_field_correctness
                .entry(field_path)
                .or_insert_with(FieldStats::empty)
                .record(is_correct);
        }
        self.per_tool
            .entry(ground_truth_tool)
            .or_insert_with(|| ToolCorrectnessStats::empty_for(ground_truth_tool))
            .record(action_correct, failed_fields);
    }

    fn record_shape_invalid(&mut self, count: usize) {
        self.shape_invalid_count += count;
        self.total += count;
    }

    fn build(self) -> ActionCorrectnessReport {
        ActionCorrectnessReport::new(
            self.total,
            self.action_correct_count,
            self.tool_selection_correct_count,
            self.shape_invalid_count,
            self.per_field_correctness,
            self.per_tool,
        )
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
    use operator_evaluation_domain::prediction::predicted_action::PredictedAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
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
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    fn inspect_action(memory: &str) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse(memory).unwrap()),
        )))
    }

    fn inspect_pair(id: &str, predicted_memory: &str) -> EvaluationPair {
        let target = MemoryRef::parse("node:1").unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        let gt_action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        let trajectory = TrainingTrajectory::new(
            TrainingTrajectoryId::parse(id).unwrap(),
            StepId::parse("s:1").unwrap(),
            AboutId::parse("about:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            TrajectoryGoal::parse("Inspect node.").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            gt_action,
        )
        .unwrap();
        let prediction =
            PredictedAction::new(trajectory.id().clone(), inspect_action(predicted_memory));
        EvaluationPair::new(trajectory, prediction).unwrap()
    }

    #[test]
    fn empty_input_reports_zero_total() {
        let report = EvaluateActionCorrectnessUseCase::new().execute(&[], &[]);
        assert_eq!(report.total(), 0);
        assert!(report.action_correctness_rate().abs() < f64::EPSILON);
    }

    #[test]
    fn shape_violations_count_against_denominator() {
        let pair = inspect_pair("t:1", "node:1");
        let violation = ShapeViolationRecord::new(1, None, "bad").unwrap();
        let report = EvaluateActionCorrectnessUseCase::new().execute(&[pair], &[violation]);

        assert_eq!(report.total(), 2);
        assert_eq!(report.action_correct_count(), 1);
        assert_eq!(report.shape_invalid_count(), 1);
        assert!((report.action_correctness_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn same_tool_wrong_field_counts_tool_selection_but_not_action_correctness() {
        let pair = inspect_pair("t:1", "node:2");
        let report = EvaluateActionCorrectnessUseCase::new().execute(&[pair], &[]);

        assert_eq!(report.total(), 1);
        assert_eq!(report.tool_selection_correct_count(), 1);
        assert_eq!(report.action_correct_count(), 0);
        assert_eq!(
            report
                .per_field_correctness()
                .keys()
                .next()
                .unwrap()
                .as_str(),
            "target"
        );
    }

    #[test]
    fn wrong_tool_is_not_tool_selection_correct() {
        let base_pair = inspect_pair("t:1", "node:1");
        let trajectory = base_pair.ground_truth().clone();
        let prediction = PredictedAction::new(
            trajectory.id().clone(),
            OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
                AskArguments::new("why").unwrap(),
            ))),
        );
        let pair = EvaluationPair::new(trajectory, prediction).unwrap();
        let report = EvaluateActionCorrectnessUseCase::new().execute(&[pair], &[]);

        assert_eq!(report.tool_selection_correct_count(), 0);
        assert_eq!(report.action_correct_count(), 0);
    }
}
