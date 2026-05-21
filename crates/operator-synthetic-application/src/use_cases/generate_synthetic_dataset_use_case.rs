//! `GenerateSyntheticDatasetUseCase` walks every case in a blueprint,
//! delegates generation to a `SyntheticCaseGenerator` adapter and
//! assembles a `SyntheticDatasetGenerationReport`.
//!
//! Per-case minimum enforcement is **not** a hard error: the report
//! exposes `every_case_satisfies_minimum()` so the caller can decide
//! whether partial coverage is acceptable. Adapter failures and domain
//! constructor failures propagate immediately.

use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_synthetic_domain::case::synthetic_case_generation_metric::SyntheticCaseGenerationMetric;
use operator_synthetic_domain::dataset::synthetic_dataset::SyntheticDataset;
use operator_synthetic_domain::dataset::synthetic_dataset_blueprint::SyntheticDatasetBlueprint;
use operator_synthetic_domain::dataset::synthetic_dataset_generation_report::SyntheticDatasetGenerationReport;

use crate::error::generate_synthetic_dataset_error::GenerateSyntheticDatasetError;
use crate::ports::synthetic_case_generator::SyntheticCaseGenerator;

#[derive(Debug)]
pub struct GenerateSyntheticDatasetUseCase<G: SyntheticCaseGenerator> {
    generator: G,
}

impl<G: SyntheticCaseGenerator> GenerateSyntheticDatasetUseCase<G> {
    pub fn new(generator: G) -> Self {
        Self { generator }
    }

    pub fn execute(
        &self,
        blueprint: &SyntheticDatasetBlueprint,
    ) -> Result<SyntheticDatasetGenerationReport, GenerateSyntheticDatasetError> {
        let mut all_trajectories = Vec::new();
        let mut case_metrics = Vec::with_capacity(blueprint.cases().len());
        for spec in blueprint.cases() {
            let trajectories = self.generator.generate(spec)?;
            let generated = ExampleCount::new(trajectories.len());
            case_metrics.push(SyntheticCaseGenerationMetric::new(
                spec.case_id().clone(),
                spec.minimum_examples(),
                generated,
            ));
            all_trajectories.extend(trajectories);
        }
        let dataset = SyntheticDataset::new(blueprint.dataset_id().clone(), all_trajectories)?;
        Ok(SyntheticDatasetGenerationReport::new(dataset, case_metrics))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::dataset_id::DatasetId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;

    use crate::error::generate_synthetic_case_error::GenerateSyntheticCaseError;

    /// Test double that emits `spec.minimum_examples()` copies of a
    /// canonical Inspect trajectory regardless of the spec's
    /// capability. The use case is supposed to be capability-agnostic;
    /// the in-memory adapter in synthetic-infra is the one that maps a
    /// capability to a fixture.
    #[derive(Debug)]
    struct StubGenerator;

    impl SyntheticCaseGenerator for StubGenerator {
        fn generate(
            &self,
            spec: &SyntheticCaseSpec,
        ) -> Result<Vec<TrainingTrajectory>, GenerateSyntheticCaseError> {
            let count = spec.minimum_examples().as_usize();
            let target = MemoryRef::parse("node:stub").expect("static");
            let visible =
                VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
            let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
                InspectArguments::new(target),
            )));
            let mut out = Vec::with_capacity(count);
            for index in 0..count {
                let id =
                    TrainingTrajectoryId::parse(format!("{}:{index}", spec.case_id().as_str()))
                        .expect("static");
                out.push(
                    TrainingTrajectory::new(
                        id,
                        StepId::parse(format!("s:{index}")).expect("static"),
                        AboutId::parse(format!("a:{index}")).expect("static"),
                        OperatorMode::Read,
                        TaskFamily::parse("test.case").expect("static"),
                        operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                            "Execute the synthetic test case.",
                        )
                        .expect("static"),
                        AllowedTools::for_mode(OperatorMode::Read),
                        visible.clone(),
                        action.clone(),
                    )
                    .expect("static"),
                );
            }
            Ok(out)
        }
    }

    #[derive(Debug)]
    struct AlwaysFailingGenerator;

    impl SyntheticCaseGenerator for AlwaysFailingGenerator {
        fn generate(
            &self,
            spec: &SyntheticCaseSpec,
        ) -> Result<Vec<TrainingTrajectory>, GenerateSyntheticCaseError> {
            Err(GenerateSyntheticCaseError::Generator {
                adapter: "stub_failing",
                case_id: spec.case_id().as_str().to_string(),
                message: "intentional".to_string(),
            })
        }
    }

    fn blueprint_with(cases: Vec<(&str, KmpMcpCapability, usize)>) -> SyntheticDatasetBlueprint {
        let case_specs = cases
            .into_iter()
            .map(|(id, capability, minimum)| {
                SyntheticCaseSpec::new(
                    SyntheticCaseId::parse(id).unwrap(),
                    capability,
                    PositiveCount::parse(minimum, "minimum").unwrap(),
                )
            })
            .collect();
        SyntheticDatasetBlueprint::new(DatasetId::parse("dataset:test").unwrap(), case_specs)
            .unwrap()
    }

    #[test]
    fn aggregates_trajectories_and_metrics_across_cases() {
        let blueprint = blueprint_with(vec![
            ("case:a", KmpMcpCapability::Inspect, 2),
            ("case:b", KmpMcpCapability::Ask, 3),
            ("case:c", KmpMcpCapability::Wake, 1),
        ]);
        let use_case = GenerateSyntheticDatasetUseCase::new(StubGenerator);

        let report = use_case.execute(&blueprint).unwrap();

        assert_eq!(report.dataset().trajectories().len(), 6);
        assert_eq!(report.case_metrics().len(), 3);
        for metric in report.case_metrics() {
            assert!(metric.satisfies_minimum());
        }
        assert!(report.every_case_satisfies_minimum());
    }

    #[test]
    fn per_case_metric_records_actual_generated_count() {
        let blueprint = blueprint_with(vec![("case:single", KmpMcpCapability::Inspect, 4)]);
        let use_case = GenerateSyntheticDatasetUseCase::new(StubGenerator);

        let report = use_case.execute(&blueprint).unwrap();

        assert_eq!(report.case_metrics().len(), 1);
        let metric = &report.case_metrics()[0];
        assert_eq!(metric.case_id().as_str(), "case:single");
        assert_eq!(metric.generated().as_usize(), 4);
        assert_eq!(metric.expected_minimum().as_usize(), 4);
    }

    #[test]
    fn generator_failure_propagates_as_case_error() {
        let blueprint = blueprint_with(vec![("case:doomed", KmpMcpCapability::Inspect, 1)]);
        let use_case = GenerateSyntheticDatasetUseCase::new(AlwaysFailingGenerator);

        let err = use_case.execute(&blueprint).unwrap_err();
        match err {
            GenerateSyntheticDatasetError::Case(GenerateSyntheticCaseError::Generator {
                adapter,
                case_id,
                ..
            }) => {
                assert_eq!(adapter, "stub_failing");
                assert_eq!(case_id, "case:doomed");
            }
            other => panic!("expected Case(Generator) error, got {other:?}"),
        }
    }
}
