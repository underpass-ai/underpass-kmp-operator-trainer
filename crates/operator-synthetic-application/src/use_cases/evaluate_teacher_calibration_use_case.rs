//! Use case that evaluates teacher policy against calibration cases.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_synthetic_domain::calibration::calibration_capability::CalibrationCapability;
use operator_synthetic_domain::calibration::calibration_case::CalibrationCase;

use crate::error::evaluate_teacher_calibration_error::EvaluateTeacherCalibrationError;
use crate::error::teacher_policy_error::TeacherPolicyError;
use crate::ports::calibration_episode_source::CalibrationEpisodeSource;
use crate::ports::teacher_policy::TeacherPolicy;
use crate::use_cases::teacher_calibration_case_result::TeacherCalibrationCaseResult;
use crate::use_cases::teacher_calibration_prediction_outcome::TeacherCalibrationPredictionOutcome;
use crate::use_cases::teacher_calibration_report::TeacherCalibrationReport;

#[derive(Debug)]
pub struct EvaluateTeacherCalibrationUseCase<S, T> {
    source: S,
    teacher: T,
}

impl<S, T> EvaluateTeacherCalibrationUseCase<S, T>
where
    S: CalibrationEpisodeSource,
    T: TeacherPolicy,
{
    pub fn new(source: S, teacher: T) -> Self {
        Self { source, teacher }
    }

    pub fn execute(&self) -> Result<TeacherCalibrationReport, EvaluateTeacherCalibrationError> {
        let cases = self.source.read()?;
        let validator = CompositeActionContractValidator::default_strict();
        let mut results = Vec::with_capacity(cases.len());
        for case in &cases {
            results.push(self.evaluate_case(case, &validator)?);
        }
        Ok(TeacherCalibrationReport::from_case_results(results))
    }

    fn evaluate_case(
        &self,
        case: &CalibrationCase,
        validator: &CompositeActionContractValidator,
    ) -> Result<TeacherCalibrationCaseResult, EvaluateTeacherCalibrationError> {
        match self.teacher.decide(case.subject()) {
            Ok(action) => Ok(result_for_action(case, &action, validator)),
            Err(TeacherPolicyError::Shape { message, .. }) => {
                Ok(TeacherCalibrationCaseResult::shape_failure(
                    case.case_id().clone(),
                    case.capability(),
                    case.category(),
                    Some(case.expected_action_rationale().clone()),
                    Some(
                        NonEmptyString::parse(message, "teacher_shape_failure").unwrap_or_else(
                            |_| {
                                NonEmptyString::parse(
                                    "teacher produced invalid action shape",
                                    "teacher_shape_failure",
                                )
                                .unwrap()
                            },
                        ),
                    ),
                ))
            }
            Err(err) => Err(err.into()),
        }
    }
}

fn result_for_action(
    case: &CalibrationCase,
    action: &OperatorAction,
    validator: &CompositeActionContractValidator,
) -> TeacherCalibrationCaseResult {
    let matched = case.accepted_actions().contains(action);
    let predicted_capability = CalibrationCapability::from_action(action);
    let tool_matched = predicted_capability == case.capability();
    let contract_valid = validator
        .validate(
            action,
            case.subject().about(),
            case.subject().mode(),
            case.subject().visible_state(),
        )
        .is_ok();
    let expected_action_rationale = if matched {
        None
    } else {
        Some(case.expected_action_rationale().clone())
    };
    TeacherCalibrationCaseResult::prediction(
        case.case_id().clone(),
        case.capability(),
        case.category(),
        TeacherCalibrationPredictionOutcome::new(matched, tool_matched, contract_valid),
        expected_action_rationale,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::error::calibration_episode_source_error::CalibrationEpisodeSourceError;
    use operator_shared_domain::action::escalate_action::EscalateAction;
    use operator_shared_domain::action::escalate_reason::EscalateReason;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::cursor::cursor::Cursor;
    use operator_shared_domain::cursor::ref_cursor::RefCursor;
    use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
    use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
    use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool::kernel_tool::KernelTool;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
    use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
    use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
    use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
    use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
    use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
    use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
    use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
    use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
    use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
    use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::model_id::ModelId;
    use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::string_map::StringMap;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_domain::calibration::accepted_actions::AcceptedActions;
    use operator_synthetic_domain::calibration::calibration_case_category::CalibrationCaseCategory;
    use operator_synthetic_domain::calibration::calibration_case_id::CalibrationCaseId;
    use operator_synthetic_domain::calibration::calibration_domain_theme::CalibrationDomainTheme;
    use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
    use operator_synthetic_domain::calibration::expected_action_rationale::ExpectedActionRationale;

    #[derive(Debug)]
    struct StubSource {
        cases: Vec<CalibrationCase>,
    }

    impl CalibrationEpisodeSource for StubSource {
        fn read(&self) -> Result<Vec<CalibrationCase>, CalibrationEpisodeSourceError> {
            Ok(self.cases.clone())
        }
    }

    #[derive(Debug)]
    struct FailingSource;

    impl CalibrationEpisodeSource for FailingSource {
        fn read(&self) -> Result<Vec<CalibrationCase>, CalibrationEpisodeSourceError> {
            Err(CalibrationEpisodeSourceError::SourceUnavailable {
                adapter: "stub",
                message: "missing".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct QueueTeacher {
        actions: Mutex<Vec<Result<OperatorAction, TeacherPolicyError>>>,
    }

    impl QueueTeacher {
        fn new(actions: Vec<Result<OperatorAction, TeacherPolicyError>>) -> Self {
            Self {
                actions: Mutex::new(actions),
            }
        }
    }

    impl TeacherPolicy for QueueTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<OperatorAction, TeacherPolicyError> {
            self.actions.lock().unwrap().remove(0)
        }
    }

    #[test]
    fn evaluates_clean_calibration_returns_passed_report() {
        let cases = all_capability_cases();
        let actions = cases
            .iter()
            .map(|case| Ok(case.accepted_actions().as_slice()[0].clone()))
            .collect();
        let report = EvaluateTeacherCalibrationUseCase::new(
            StubSource { cases },
            QueueTeacher::new(actions),
        )
        .execute()
        .unwrap();
        assert!(report.gate_passed());
        assert!((report.overall_accuracy() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluates_failing_calibration_below_overall_threshold_returns_failed() {
        let cases = all_capability_cases();
        let actions = cases.iter().map(|_| Ok(escalate())).collect();
        let report = EvaluateTeacherCalibrationUseCase::new(
            StubSource { cases },
            QueueTeacher::new(actions),
        )
        .execute()
        .unwrap();
        assert!(!report.gate_passed());
        assert!(report.overall_accuracy() < 0.80);
    }

    #[test]
    fn evaluates_failing_calibration_below_per_capability_floor_returns_failed() {
        let cases = all_capability_cases();
        let actions = cases
            .iter()
            .map(|case| {
                if case.capability() == CalibrationCapability::KernelInspect {
                    Ok(wake())
                } else {
                    Ok(case.accepted_actions().as_slice()[0].clone())
                }
            })
            .collect();
        let report = EvaluateTeacherCalibrationUseCase::new(
            StubSource { cases },
            QueueTeacher::new(actions),
        )
        .execute()
        .unwrap();
        assert!(!report.gate_passed());
        assert!(
            report
                .gate_failure_reason()
                .unwrap()
                .as_str()
                .contains("kernel_inspect")
        );
    }

    #[test]
    fn propagates_source_error() {
        let use_case =
            EvaluateTeacherCalibrationUseCase::new(FailingSource, QueueTeacher::new(vec![]));
        assert!(matches!(
            use_case.execute(),
            Err(EvaluateTeacherCalibrationError::Source(_))
        ));
    }

    #[test]
    fn propagates_teacher_shape_failure_as_shape_failed_count() {
        let cases = vec![case_for(inspect())];
        let report = EvaluateTeacherCalibrationUseCase::new(
            StubSource { cases },
            QueueTeacher::new(vec![Err(TeacherPolicyError::Shape {
                adapter: "stub",
                message: "bad json".to_string(),
            })]),
        )
        .execute()
        .unwrap();
        assert_eq!(report.shape_failed_count().as_usize(), 1);
        assert!(!report.gate_passed());
    }

    fn all_capability_cases() -> Vec<CalibrationCase> {
        vec![
            case_for(ingest()),
            case_for(wake()),
            case_for(ask()),
            case_for(near()),
            case_for(goto()),
            case_for(rewind()),
            case_for(forward()),
            case_for(trace()),
            case_for(inspect()),
            case_for(write_memory()),
            case_for(stop()),
            case_for(escalate()),
        ]
    }

    fn case_for(action: OperatorAction) -> CalibrationCase {
        let capability = CalibrationCapability::from_action(&action);
        CalibrationCase::new(
            CalibrationCaseId::parse(format!("calib:{}", capability.as_str())).unwrap(),
            CalibrationDomainTheme::TechnicalIncident,
            CalibrationCaseCategory::Happy,
            subject_for_action(&action),
            AcceptedActions::new(vec![action]).unwrap(),
            ExpectedActionRationale::parse("The expected action is explicit.").unwrap(),
        )
    }

    fn subject_for_action(action: &OperatorAction) -> CalibrationSubject {
        let mode = match action.tool() {
            Some(KernelTool::Ingest | KernelTool::WriteMemory) => OperatorMode::Write,
            _ => OperatorMode::Read,
        };
        let target = MemoryRef::parse("node:visible").unwrap();
        let other = MemoryRef::parse("node:other").unwrap();
        let dimension = DimensionRef::parse("agent:operator").unwrap();
        let active_cursor = match action.tool() {
            Some(KernelTool::Rewind | KernelTool::Forward) => {
                Some(Cursor::Temporal(temporal_cursor()))
            }
            _ => None,
        };
        CalibrationSubject::new(
            AboutId::parse("about:calibration").unwrap(),
            mode,
            TaskFamily::parse("calibration.test").unwrap(),
            TrajectoryGoal::parse("Select the expected action.").unwrap(),
            AllowedTools::for_mode(mode),
            VisibleState::assemble(
                [target, other],
                [dimension],
                active_cursor,
                BudgetSnapshot::bounded(4, 1024),
            ),
        )
        .unwrap()
    }

    fn wake() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
            WakeArguments::new(AboutId::parse("about:calibration").unwrap()),
        )))
    }

    fn ask() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
            AskArguments::new("What is known?").unwrap(),
        )))
    }

    fn near() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
            NearArguments::new(
                MemoryRef::parse("node:visible").unwrap(),
                vec![DimensionRef::parse("agent:operator").unwrap()],
                Some(count(3)),
            )
            .unwrap(),
        )))
    }

    fn goto() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Ref(RefCursor::new(
                MemoryRef::parse("node:visible").unwrap(),
            ))),
        )))
    }

    fn rewind() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Rewind(
            RewindArguments::new(temporal_cursor(), count(2)),
        )))
    }

    fn forward() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
            ForwardArguments::new(temporal_cursor(), count(2)),
        )))
    }

    fn trace() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
            TraceArguments::new(
                MemoryRef::parse("node:visible").unwrap(),
                Some(MemoryRef::parse("node:other").unwrap()),
                count(8),
            ),
        )))
    }

    fn inspect() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:visible").unwrap()),
        )))
    }

    fn ingest() -> OperatorAction {
        let dimension = DimensionRef::parse("agent:operator").unwrap();
        let entry = MemoryRef::parse("node:calibration:new").unwrap();
        let coordinate = IngestTemporalCoordinate::new(
            dimension.clone(),
            NonEmptyString::parse("scope:writer", "scope").unwrap(),
            None,
            None,
            None,
            None,
            None,
            Some(count(1)),
            None,
            StringMap::empty(),
        )
        .unwrap();
        let memory = IngestMemory::new(
            vec![IngestDimension::new(
                dimension,
                NonEmptyString::parse("agent", "kind").unwrap(),
                Some(NonEmptyString::parse("Writer", "title").unwrap()),
                StringMap::empty(),
            )],
            vec![
                IngestEntry::new(
                    entry,
                    NonEmptyString::parse("decision", "kind").unwrap(),
                    NonEmptyString::parse("Calibrated write.", "text").unwrap(),
                    vec![coordinate],
                    StringMap::empty(),
                )
                .unwrap(),
            ],
            vec![],
            vec![],
        )
        .unwrap();
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ingest(
            IngestArguments::new(
                AboutId::parse("about:calibration").unwrap(),
                memory,
                None,
                NonEmptyString::parse("idem:calibration", "idempotency_key").unwrap(),
                true,
            ),
        )))
    }

    fn write_memory() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
            WriteMemoryArguments::new(
                "Record calibrated memory.",
                "The calibrated write is ready.",
                vec![MemoryRef::parse("node:visible").unwrap()],
            )
            .unwrap(),
        )))
    }

    fn stop() -> OperatorAction {
        OperatorAction::Stop(
            StopAction::new(StopReason::AnswerReady, Some("done".to_string()), vec![]).unwrap(),
        )
    }

    fn escalate() -> OperatorAction {
        OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::BeyondCapability,
            ModelId::parse("frontier-reasoner").unwrap(),
        ))
    }

    fn temporal_cursor() -> TemporalCursor {
        TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse("seq:1").unwrap(),
        )
    }

    fn count(value: usize) -> PositiveCount {
        PositiveCount::parse(value, "calibration").unwrap()
    }
}
