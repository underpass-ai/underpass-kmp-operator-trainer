//! Use case that asks a calibrated teacher to build a realistic corpus while
//! dropping bad rows with auditable reasons.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::error::build_realistic_corpus_error::BuildRealisticCorpusError;
use crate::error::teacher_policy_error::TeacherPolicyError;
use crate::ports::corpus_event_sink::CorpusEventSink;
use crate::ports::scenario::Scenario;
use crate::ports::scenario_source::ScenarioSource;
use crate::ports::teacher_policy::TeacherPolicy;
use crate::use_cases::drop_entry::DropEntry;
use crate::use_cases::drop_reason::DropReason;
use crate::use_cases::max_drop_rate::MaxDropRate;
use crate::use_cases::realistic_corpus_report::RealisticCorpusReport;

#[derive(Debug)]
struct DropCandidate {
    reason: DropReason,
    predicted_action: Option<OperatorAction>,
    teacher_finish_reason: Option<FinishReason>,
}

impl DropCandidate {
    fn with_decision(
        reason: DropReason,
        decision: &operator_synthetic_domain::calibration::teacher_decision::TeacherDecision,
    ) -> Box<Self> {
        Box::new(Self {
            reason,
            predicted_action: Some(decision.action().clone()),
            teacher_finish_reason: Some(decision.finish_reason()),
        })
    }

    fn without_action(
        reason: DropReason,
        teacher_finish_reason: Option<FinishReason>,
    ) -> Box<Self> {
        Box::new(Self {
            reason,
            predicted_action: None,
            teacher_finish_reason,
        })
    }
}

#[derive(Debug)]
pub struct BuildRealisticCorpusUseCase<S, T, V, E> {
    scenarios: S,
    teacher: T,
    validator: V,
    event_sink: E,
}

impl<S, T, V, E> BuildRealisticCorpusUseCase<S, T, V, E>
where
    S: ScenarioSource,
    T: TeacherPolicy,
    V: ActionContractValidator,
    E: CorpusEventSink,
{
    pub fn new(scenarios: S, teacher: T, validator: V, event_sink: E) -> Self {
        Self {
            scenarios,
            teacher,
            validator,
            event_sink,
        }
    }

    pub fn execute(
        &self,
        max_drop_rate: MaxDropRate,
        limit: Option<PositiveCount>,
    ) -> Result<RealisticCorpusReport, BuildRealisticCorpusError> {
        let mut scenarios = self.scenarios.read()?;
        if let Some(limit) = limit {
            scenarios.truncate(limit.as_usize());
        }
        self.event_sink.on_run_started(scenarios.len())?;
        let mut accepted = Vec::new();
        let mut dropped = Vec::new();
        for (index, scenario) in scenarios.iter().enumerate() {
            match self.process_one(scenario, index) {
                Ok(trajectory) => {
                    self.event_sink
                        .on_row_accepted(index, scenario, &trajectory)?;
                    accepted.push(trajectory);
                }
                Err(candidate) => {
                    let candidate = *candidate;
                    let drop = DropEntry::new(
                        scenario.id().clone(),
                        scenario.target(),
                        candidate.reason,
                        candidate.predicted_action,
                        scenario.subject_hash().clone(),
                        candidate.teacher_finish_reason,
                    );
                    self.event_sink.on_row_dropped(index, scenario, &drop)?;
                    dropped.push(drop);
                }
            }
        }
        let report = RealisticCorpusReport::from_outcome(accepted, dropped, max_drop_rate);
        self.event_sink.on_run_finished(&report)?;
        Ok(report)
    }

    fn process_one(
        &self,
        scenario: &Scenario,
        index: usize,
    ) -> Result<TrainingTrajectory, Box<DropCandidate>> {
        let decision = self
            .teacher
            .decide(scenario.subject())
            .map_err(drop_from_teacher_error)?;
        let action = decision.action().clone();
        Self::validate_target(scenario, &action)
            .map_err(|reason| DropCandidate::with_decision(reason, &decision))?;
        scenario
            .acceptance_criteria()
            .matches(&action)
            .map_err(|violation| {
                DropCandidate::with_decision(DropReason::SemanticMismatch { violation }, &decision)
            })?;
        self.validator
            .validate(
                decision.action(),
                scenario.subject().about(),
                scenario.subject().mode(),
                scenario.subject().visible_state(),
            )
            .map_err(|violations| {
                DropCandidate::with_decision(
                    DropReason::ContractViolation { violations },
                    &decision,
                )
            })?;
        Self::build_trajectory(scenario, index, action)
            .map_err(|reason| DropCandidate::with_decision(reason, &decision))
    }

    fn validate_target(scenario: &Scenario, action: &OperatorAction) -> Result<(), DropReason> {
        if scenario.target().matches_action(action) {
            return Ok(());
        }
        Err(DropReason::TargetMismatch {
            expected: scenario.target(),
            got_kind: action_label(action),
        })
    }

    fn build_trajectory(
        scenario: &Scenario,
        index: usize,
        action: OperatorAction,
    ) -> Result<TrainingTrajectory, DropReason> {
        let id = TrainingTrajectoryId::parse(format!("{}:trajectory:{index:04}", scenario.id()))
            .map_err(|err| DropReason::TrajectoryBuild {
                message: err.to_string(),
            })?;
        let step_id =
            StepId::parse(format!("{}:step:{index:04}", scenario.id())).map_err(|err| {
                DropReason::TrajectoryBuild {
                    message: err.to_string(),
                }
            })?;
        TrainingTrajectory::new(
            id,
            step_id,
            scenario.subject().about().clone(),
            scenario.subject().mode(),
            scenario.subject().task_family().clone(),
            scenario.subject().goal().clone(),
            scenario.subject().allowed_tools().clone(),
            scenario.subject().visible_state().clone(),
            action,
        )
        .map_err(|err| DropReason::TrajectoryBuild {
            message: err.to_string(),
        })
    }
}

fn drop_from_teacher_error(err: TeacherPolicyError) -> Box<DropCandidate> {
    match err {
        TeacherPolicyError::Shape {
            message,
            finish_reason,
            ..
        } => DropCandidate::without_action(DropReason::ParseFailure { message }, finish_reason),
        TeacherPolicyError::TruncatedResponse {
            finish_reason,
            content_len,
            raw_content_tail,
            request_bytes,
            response_bytes,
            elapsed_ms,
            request_id,
            ..
        } => DropCandidate::without_action(
            DropReason::TeacherTruncation {
                finish_reason,
                content_len,
                raw_content_tail,
                request_bytes,
                response_bytes,
                elapsed_ms,
                request_id,
            },
            Some(finish_reason),
        ),
        other => DropCandidate::without_action(
            DropReason::TeacherError {
                message: other.to_string(),
            },
            None,
        ),
    }
}

fn action_label(action: &OperatorAction) -> String {
    action.tool().map_or_else(
        || action.kind().as_str().to_string(),
        |tool| tool.as_str().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::subject_hash::SubjectHash;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
    use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_acceptance_criteria::SyntheticAcceptanceCriteria;
    use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

    use crate::error::corpus_event_sink_error::CorpusEventSinkError;
    use crate::error::scenario_source_error::ScenarioSourceError;
    use crate::error::teacher_policy_error::TeacherPolicyError;
    use crate::ports::corpus_event_sink::CorpusEventSink;
    use crate::ports::scenario_id::ScenarioId;

    #[derive(Debug)]
    struct StubSource {
        scenarios: Vec<Scenario>,
    }

    impl ScenarioSource for StubSource {
        fn read(&self) -> Result<Vec<Scenario>, ScenarioSourceError> {
            Ok(self.scenarios.clone())
        }
    }

    #[derive(Debug)]
    struct StubTeacher {
        action: Result<OperatorAction, TeacherPolicyError>,
    }

    impl TeacherPolicy for StubTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<TeacherDecision, TeacherPolicyError> {
            self.action
                .clone()
                .map(|action| TeacherDecision::new(action, FinishReason::Stop, subject_hash()))
        }
    }

    #[derive(Debug, Default)]
    struct RecordingSink {
        events: std::sync::Mutex<Vec<&'static str>>,
    }

    impl RecordingSink {
        fn events(&self) -> Vec<&'static str> {
            self.events.lock().unwrap().clone()
        }
    }

    impl CorpusEventSink for std::sync::Arc<RecordingSink> {
        fn on_run_started(&self, _total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
            self.events.lock().unwrap().push("started");
            Ok(())
        }

        fn on_row_accepted(
            &self,
            _index: usize,
            _scenario: &Scenario,
            _trajectory: &TrainingTrajectory,
        ) -> Result<(), CorpusEventSinkError> {
            self.events.lock().unwrap().push("accepted");
            Ok(())
        }

        fn on_row_dropped(
            &self,
            _index: usize,
            _scenario: &Scenario,
            _drop: &DropEntry,
        ) -> Result<(), CorpusEventSinkError> {
            self.events.lock().unwrap().push("dropped");
            Ok(())
        }

        fn on_run_finished(
            &self,
            _report: &RealisticCorpusReport,
        ) -> Result<(), CorpusEventSinkError> {
            self.events.lock().unwrap().push("finished");
            Ok(())
        }
    }

    #[test]
    fn all_scenarios_accepted_returns_passed_gate() {
        let report = use_case(vec![inspect_scenario()], inspect_action())
            .execute(MaxDropRate::parse(0.0).unwrap(), None)
            .unwrap();
        assert!(report.gate_passed());
        assert_eq!(report.accepted_count(), 1);
        assert_eq!(report.dropped_count(), 0);
    }

    #[test]
    fn one_scenario_dropped_within_max_drop_rate_returns_passed_gate() {
        let report = use_case(vec![inspect_scenario()], wake_action())
            .execute(MaxDropRate::parse(1.0).unwrap(), None)
            .unwrap();
        assert!(report.gate_passed());
        assert_eq!(report.dropped_count(), 1);
    }

    #[test]
    fn drop_rate_above_threshold_returns_failed_gate() {
        let report = use_case(vec![inspect_scenario()], wake_action())
            .execute(MaxDropRate::parse(0.05).unwrap(), None)
            .unwrap();
        assert!(!report.gate_passed());
        assert!(report.gate_failure_reason().unwrap().contains("drop_rate"));
    }

    #[test]
    fn teacher_error_propagated_as_drop_entry() {
        let use_case = BuildRealisticCorpusUseCase::new(
            StubSource {
                scenarios: vec![inspect_scenario()],
            },
            StubTeacher {
                action: Err(TeacherPolicyError::Transport {
                    adapter: "stub",
                    message: "offline".to_string(),
                }),
            },
            CompositeActionContractValidator::default_strict(),
            std::sync::Arc::new(RecordingSink::default()),
        );
        let report = use_case
            .execute(MaxDropRate::parse(1.0).unwrap(), None)
            .unwrap();
        assert!(matches!(
            report.dropped()[0].reason(),
            DropReason::TeacherError { .. }
        ));
        assert!(report.dropped()[0].predicted_action().is_none());
    }

    #[test]
    fn teacher_truncation_propagated_as_drop_entry() {
        let use_case = BuildRealisticCorpusUseCase::new(
            StubSource {
                scenarios: vec![inspect_scenario()],
            },
            StubTeacher {
                action: Err(TeacherPolicyError::TruncatedResponse {
                    adapter: "stub",
                    finish_reason: FinishReason::Length,
                    content_len: 4205,
                    raw_content_tail: "\"partial\":true".to_string(),
                    request_bytes: 1234,
                    response_bytes: 4321,
                    elapsed_ms: 99,
                    request_id: Some("req_test".to_string()),
                }),
            },
            CompositeActionContractValidator::default_strict(),
            std::sync::Arc::new(RecordingSink::default()),
        );

        let report = use_case
            .execute(MaxDropRate::parse(1.0).unwrap(), None)
            .unwrap();

        assert!(matches!(
            report.dropped()[0].reason(),
            DropReason::TeacherTruncation {
                finish_reason: FinishReason::Length,
                content_len: 4205,
                raw_content_tail: _,
                request_bytes: 1234,
                response_bytes: 4321,
                elapsed_ms: 99,
                request_id: Some(_)
            }
        ));
        assert!(report.dropped()[0].predicted_action().is_none());
        assert_eq!(
            report.dropped()[0].teacher_finish_reason(),
            Some(FinishReason::Length)
        );
    }

    #[test]
    fn target_mismatch_propagated_as_drop_entry() {
        let report = use_case(vec![inspect_scenario()], wake_action())
            .execute(MaxDropRate::parse(1.0).unwrap(), None)
            .unwrap();
        assert!(matches!(
            report.dropped()[0].reason(),
            DropReason::TargetMismatch { .. }
        ));
        assert!(report.dropped()[0].predicted_action().is_some());
        assert_eq!(report.dropped()[0].subject_hash(), &subject_hash());
        assert_eq!(
            report.dropped()[0].teacher_finish_reason(),
            Some(FinishReason::Stop)
        );
    }

    #[test]
    fn semantic_mismatch_propagated_as_drop_entry() {
        let scenario = stop_scenario(SyntheticAcceptanceCriteria::new(
            Some(operator_shared_domain::action::stop_reason::StopReason::NoCandidate),
            None,
        ));
        let report = use_case(vec![scenario], stop_answer_ready())
            .execute(MaxDropRate::parse(1.0).unwrap(), None)
            .unwrap();

        assert!(matches!(
            report.dropped()[0].reason(),
            DropReason::SemanticMismatch { .. }
        ));
        assert!(report.dropped()[0].predicted_action().is_some());
    }

    #[test]
    fn permissive_criteria_allows_any_stop_reason() {
        let report = use_case(
            vec![stop_scenario(SyntheticAcceptanceCriteria::permissive())],
            stop_answer_ready(),
        )
        .execute(MaxDropRate::parse(0.0).unwrap(), None)
        .unwrap();

        assert!(report.gate_passed());
        assert_eq!(report.accepted_count(), 1);
    }

    #[test]
    fn event_sink_receives_started_row_and_finished_events() {
        let sink = std::sync::Arc::new(RecordingSink::default());
        let report = use_case_with_sink(vec![inspect_scenario()], inspect_action(), sink.clone())
            .execute(MaxDropRate::parse(0.0).unwrap(), None)
            .unwrap();

        assert!(report.gate_passed());
        assert_eq!(sink.events(), vec!["started", "accepted", "finished"]);
    }

    fn use_case(
        scenarios: Vec<Scenario>,
        action: OperatorAction,
    ) -> BuildRealisticCorpusUseCase<
        StubSource,
        StubTeacher,
        CompositeActionContractValidator,
        std::sync::Arc<RecordingSink>,
    > {
        use_case_with_sink(
            scenarios,
            action,
            std::sync::Arc::new(RecordingSink::default()),
        )
    }

    fn use_case_with_sink(
        scenarios: Vec<Scenario>,
        action: OperatorAction,
        sink: std::sync::Arc<RecordingSink>,
    ) -> BuildRealisticCorpusUseCase<
        StubSource,
        StubTeacher,
        CompositeActionContractValidator,
        std::sync::Arc<RecordingSink>,
    > {
        BuildRealisticCorpusUseCase::new(
            StubSource { scenarios },
            StubTeacher { action: Ok(action) },
            CompositeActionContractValidator::default_strict(),
            sink,
        )
    }

    fn inspect_scenario() -> Scenario {
        let target_ref = MemoryRef::parse("node:target").unwrap();
        Scenario::new(
            ScenarioId::parse("scenario:inspect").unwrap(),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
            CalibrationSubject::new(
                AboutId::parse("about:incident").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.inspect").unwrap(),
                TrajectoryGoal::parse("Inspect the visible target.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                VisibleState::assemble([target_ref], [], None, BudgetSnapshot::bounded(3, 1024)),
                None,
            )
            .unwrap(),
            SyntheticAcceptanceCriteria::permissive(),
            subject_hash(),
        )
    }

    fn stop_scenario(criteria: SyntheticAcceptanceCriteria) -> Scenario {
        Scenario::new(
            ScenarioId::parse("scenario:stop").unwrap(),
            SyntheticGenerationTarget::Stop,
            CalibrationSubject::new(
                AboutId::parse("about:incident").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.stop").unwrap(),
                TrajectoryGoal::parse("No candidate remains.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                VisibleState::assemble([], [], None, BudgetSnapshot::bounded(1, 1024)),
                None,
            )
            .unwrap(),
            criteria,
            subject_hash(),
        )
    }

    fn inspect_action() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:target").unwrap()),
        )))
    }

    fn wake_action() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
            WakeArguments::new(AboutId::parse("about:incident").unwrap()),
        )))
    }

    fn stop_answer_ready() -> OperatorAction {
        OperatorAction::Stop(
            operator_shared_domain::action::stop_action::StopAction::new(
                operator_shared_domain::action::stop_reason::StopReason::AnswerReady,
                None,
                vec![],
            )
            .unwrap(),
        )
    }

    fn subject_hash() -> SubjectHash {
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap()
    }
}
