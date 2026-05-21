//! Use case that loads a corpus and evaluates its quality contract.

use operator_synthetic_domain::quality::corpus_quality_validator::CorpusQualityValidator;

use crate::error::evaluate_corpus_quality_error::EvaluateCorpusQualityError;
use crate::ports::corpus_source::CorpusSource;
use crate::use_cases::corpus_quality_report::CorpusQualityReport;

#[derive(Debug)]
pub struct EvaluateCorpusQualityUseCase<S, V> {
    source: S,
    validator: V,
}

impl<S, V> EvaluateCorpusQualityUseCase<S, V>
where
    S: CorpusSource,
    V: CorpusQualityValidator,
{
    pub fn new(source: S, validator: V) -> Self {
        Self { source, validator }
    }

    pub fn execute(&self) -> Result<CorpusQualityReport, EvaluateCorpusQualityError> {
        let snapshot = self.source.read()?;
        Ok(match self.validator.validate(&snapshot) {
            Ok(()) => CorpusQualityReport::passed(),
            Err(violations) => CorpusQualityReport::failed(violations),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::corpus_source_error::CorpusSourceError;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::contract::contract_violation::ContractViolation;
    use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::dataset_id::DatasetId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::dataset::synthetic_dataset::SyntheticDataset;
    use operator_synthetic_domain::episode::capability_target::CapabilityTarget;
    use operator_synthetic_domain::episode::episode_id::EpisodeId;
    use operator_synthetic_domain::episode::episode_objective::EpisodeObjective;
    use operator_synthetic_domain::episode::episode_step_plan::EpisodeStepPlan;
    use operator_synthetic_domain::episode::episode_theme::EpisodeTheme;
    use operator_synthetic_domain::episode::synthetic_episode_spec::SyntheticEpisodeSpec;
    use operator_synthetic_domain::quality::composite_corpus_quality_validator::CompositeCorpusQualityValidator;
    use operator_synthetic_domain::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use operator_synthetic_domain::quality::corpus_quality_validator::CorpusQualityValidator;
    use operator_synthetic_domain::quality::corpus_quality_violations::CorpusQualityViolations;
    use operator_synthetic_domain::quality::corpus_snapshot::CorpusSnapshot;
    use operator_synthetic_domain::quality::test_support::{
        clean_corpus_snapshot, corpus_failing_five_specs,
    };

    #[derive(Debug)]
    struct StubSource {
        snapshot: CorpusSnapshot,
    }

    impl CorpusSource for StubSource {
        fn read(&self) -> Result<CorpusSnapshot, CorpusSourceError> {
            Ok(self.snapshot.clone())
        }
    }

    #[derive(Debug)]
    struct FailingSource;

    impl CorpusSource for FailingSource {
        fn read(&self) -> Result<CorpusSnapshot, CorpusSourceError> {
            Err(CorpusSourceError::SourceUnavailable {
                adapter: "stub",
                message: "missing".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct AlwaysValidValidator;

    impl CorpusQualityValidator for AlwaysValidValidator {
        fn validate(&self, _snapshot: &CorpusSnapshot) -> Result<(), CorpusQualityViolations> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct AlwaysInvalidValidator;

    impl CorpusQualityValidator for AlwaysInvalidValidator {
        fn validate(&self, _snapshot: &CorpusSnapshot) -> Result<(), CorpusQualityViolations> {
            let mut violations = CorpusQualityViolations::new();
            violations.push(ContractViolation::new(
                ContractViolationCode::FrontierCeiling,
                "audit.frontier_ceiling_recorded",
                "missing",
            ));
            Err(violations)
        }
    }

    fn snapshot() -> CorpusSnapshot {
        CorpusSnapshot::new(
            SyntheticDataset::new(
                DatasetId::parse("dataset:quality").unwrap(),
                vec![trajectory()],
            )
            .unwrap(),
            vec![episode()],
            CorpusAuditSnapshot::clean(),
            None,
        )
        .unwrap()
    }

    fn trajectory() -> TrainingTrajectory {
        let target = MemoryRef::parse("node:quality").unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse("trajectory:quality").unwrap(),
            StepId::parse("step:quality").unwrap(),
            AboutId::parse("about:quality").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            TrajectoryGoal::parse("Inspect quality evidence.").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
                InspectArguments::new(target),
            ))),
        )
        .unwrap()
    }

    fn episode() -> SyntheticEpisodeSpec {
        SyntheticEpisodeSpec::new(
            EpisodeId::parse("episode:quality").unwrap(),
            EpisodeTheme::Incident,
            EpisodeObjective::parse("Evaluate corpus quality.").unwrap(),
            vec![EpisodeStepPlan::new(
                CapabilityTarget::new(
                    KmpMcpCapability::Inspect,
                    PositiveCount::parse(1, "minimum").unwrap(),
                ),
                EpisodeObjective::parse("Inspect evidence.").unwrap(),
            )],
        )
        .unwrap()
    }

    #[test]
    fn returns_valid_report_for_valid_corpus() {
        let use_case = EvaluateCorpusQualityUseCase::new(
            StubSource {
                snapshot: snapshot(),
            },
            AlwaysValidValidator,
        );
        let report = use_case.execute().unwrap();
        assert!(report.is_valid());
    }

    #[test]
    fn returns_invalid_report_for_failed_specs() {
        let use_case = EvaluateCorpusQualityUseCase::new(
            StubSource {
                snapshot: snapshot(),
            },
            AlwaysInvalidValidator,
        );
        let report = use_case.execute().unwrap();
        assert!(!report.is_valid());
        assert_eq!(report.violations().len(), 1);
    }

    #[test]
    fn propagates_source_failure() {
        let use_case = EvaluateCorpusQualityUseCase::new(FailingSource, AlwaysValidValidator);
        assert!(matches!(
            use_case.execute(),
            Err(EvaluateCorpusQualityError::Source(_))
        ));
    }

    #[test]
    fn use_case_reports_passed_on_clean_corpus() {
        let use_case = EvaluateCorpusQualityUseCase::new(
            StubSource {
                snapshot: clean_corpus_snapshot(),
            },
            CompositeCorpusQualityValidator::default_strict(),
        );
        let report = use_case.execute().unwrap();
        assert!(report.is_valid());
    }

    #[test]
    fn use_case_reports_failed_on_multi_violation_corpus() {
        let use_case = EvaluateCorpusQualityUseCase::new(
            StubSource {
                snapshot: corpus_failing_five_specs(),
            },
            CompositeCorpusQualityValidator::default_strict(),
        );
        let report = use_case.execute().unwrap();
        assert!(!report.is_valid());
        assert!(report.violations().len() >= 5);
    }

    #[test]
    fn use_case_propagates_source_error_for_seed_flow() {
        let use_case = EvaluateCorpusQualityUseCase::new(
            FailingSource,
            CompositeCorpusQualityValidator::default_strict(),
        );
        assert!(matches!(
            use_case.execute(),
            Err(EvaluateCorpusQualityError::Source(_))
        ));
    }
}
