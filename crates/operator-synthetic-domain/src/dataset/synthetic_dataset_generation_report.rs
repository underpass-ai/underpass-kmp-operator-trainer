//! Aggregated outcome of a synthetic generation run: the resulting
//! dataset plus a per-case metric, useful to caller code that needs to
//! enforce "every capability has at least N examples".

use operator_shared_domain::value_objects::example_count::ExampleCount;

use crate::case::synthetic_case_generation_metric::SyntheticCaseGenerationMetric;
use crate::dataset::synthetic_dataset::SyntheticDataset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticDatasetGenerationReport {
    dataset: SyntheticDataset,
    case_metrics: Vec<SyntheticCaseGenerationMetric>,
}

impl SyntheticDatasetGenerationReport {
    pub fn new(
        dataset: SyntheticDataset,
        case_metrics: Vec<SyntheticCaseGenerationMetric>,
    ) -> Self {
        Self {
            dataset,
            case_metrics,
        }
    }

    pub fn dataset(&self) -> &SyntheticDataset {
        &self.dataset
    }

    pub fn case_metrics(&self) -> &[SyntheticCaseGenerationMetric] {
        &self.case_metrics
    }

    pub fn total_generated(&self) -> ExampleCount {
        ExampleCount::new(
            self.case_metrics
                .iter()
                .map(|m| m.generated().as_usize())
                .sum(),
        )
    }

    pub fn every_case_satisfies_minimum(&self) -> bool {
        self.case_metrics
            .iter()
            .all(SyntheticCaseGenerationMetric::satisfies_minimum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::synthetic_case_generation_metric::SyntheticCaseGenerationMetric;
    use operator_shared_domain::ids::dataset_id::DatasetId;
    use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    fn metric(case: &str, expected: usize, generated: usize) -> SyntheticCaseGenerationMetric {
        SyntheticCaseGenerationMetric::new(
            SyntheticCaseId::parse(case).unwrap(),
            PositiveCount::parse(expected, "minimum").unwrap(),
            ExampleCount::new(generated),
        )
    }

    fn placeholder_dataset() -> SyntheticDataset {
        // The dataset itself is not under test; we only need a value here.
        SyntheticDataset::new(
            DatasetId::parse("dataset:placeholder").unwrap(),
            placeholder_trajectories(),
        )
        .unwrap()
    }

    fn placeholder_trajectories()
    -> Vec<operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory> {
        use operator_shared_domain::action::operator_action::OperatorAction;
        use operator_shared_domain::action::tool_call_action::ToolCallAction;
        use operator_shared_domain::ids::about_id::AboutId;
        use operator_shared_domain::ids::step_id::StepId;
        use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
        use operator_shared_domain::mode::allowed_tools::AllowedTools;
        use operator_shared_domain::mode::operator_mode::OperatorMode;
        use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
        use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
        use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
        use operator_shared_domain::value_objects::memory_ref::MemoryRef;
        use operator_shared_domain::value_objects::task_family::TaskFamily;
        use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
        use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
        use operator_shared_domain::visible_state::visible_state::VisibleState;

        let target = MemoryRef::parse("node:1").unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        vec![
            TrainingTrajectory::new(
                TrainingTrajectoryId::parse("t:1").unwrap(),
                StepId::parse("s:1").unwrap(),
                AboutId::parse("a:1").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.inspect").unwrap(),
                TrajectoryGoal::parse("Inspect the visible memory node.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                visible,
                action,
            )
            .unwrap(),
        ]
    }

    #[test]
    fn total_generated_sums_metrics() {
        let report = SyntheticDatasetGenerationReport::new(
            placeholder_dataset(),
            vec![metric("a", 1, 2), metric("b", 1, 3)],
        );
        assert_eq!(report.total_generated().as_usize(), 5);
    }

    #[test]
    fn every_case_satisfies_minimum_only_when_all_do() {
        let ok = SyntheticDatasetGenerationReport::new(
            placeholder_dataset(),
            vec![metric("a", 1, 2), metric("b", 1, 1)],
        );
        assert!(ok.every_case_satisfies_minimum());

        let nok = SyntheticDatasetGenerationReport::new(
            placeholder_dataset(),
            vec![metric("a", 3, 2), metric("b", 1, 1)],
        );
        assert!(!nok.every_case_satisfies_minimum());
    }
}
