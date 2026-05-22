//! Teacher-backed synthetic case generator for v7.3 trajectory production.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::string_map::StringMap;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_application::error::generate_synthetic_case_error::GenerateSyntheticCaseError;
use operator_synthetic_application::ports::synthetic_case_generator::SyntheticCaseGenerator;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::prepared_operator_action::PreparedOperatorAction;
use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

const ADAPTER: &str = "teacher_backed_synthetic_case_generator";

#[derive(Debug)]
pub struct TeacherBackedSyntheticCaseGenerator<T: TeacherPolicy> {
    teacher: T,
}

impl<T: TeacherPolicy> TeacherBackedSyntheticCaseGenerator<T> {
    pub fn new(teacher: T) -> Self {
        Self { teacher }
    }

    fn build_subject(
        spec: &SyntheticCaseSpec,
        index: usize,
    ) -> Result<CalibrationSubject, GenerateSyntheticCaseError> {
        let target = spec.target();
        let mode = target.canonical_mode();
        let about = about_for(target, index)?;
        let goal = goal_for(target)?;
        let task_family = TaskFamily::parse(format!("teacher_backed.{}", target.name()))
            .map_err(|err| generator_error(spec, err.to_string()))?;
        let (visible_state, prepared_action) = subject_state_for(target, &about, index)?;
        CalibrationSubject::new(
            about,
            mode,
            task_family,
            goal,
            AllowedTools::for_mode(mode),
            visible_state,
            prepared_action,
        )
        .map_err(|err| generator_error(spec, err.to_string()))
    }

    fn build_trajectory(
        spec: &SyntheticCaseSpec,
        index: usize,
        subject: &CalibrationSubject,
        action: OperatorAction,
    ) -> Result<TrainingTrajectory, GenerateSyntheticCaseError> {
        let trajectory_id =
            TrainingTrajectoryId::parse(format!("{}:{index:04}", spec.case_id().as_str()))
                .map_err(|err| generator_error(spec, err.to_string()))?;
        let step_id = StepId::parse(format!(
            "step:{}:{}:{index:04}",
            spec.case_id().as_str(),
            spec.target().name()
        ))
        .map_err(|err| generator_error(spec, err.to_string()))?;
        TrainingTrajectory::new(
            trajectory_id,
            step_id,
            subject.about().clone(),
            subject.mode(),
            subject.task_family().clone(),
            subject.goal().clone(),
            subject.allowed_tools().clone(),
            subject.visible_state().clone(),
            action,
        )
        .map_err(|err| generator_error(spec, err.to_string()))
    }

    fn validate_teacher_action(
        spec: &SyntheticCaseSpec,
        subject: &CalibrationSubject,
        action: &OperatorAction,
    ) -> Result<(), GenerateSyntheticCaseError> {
        let expected = spec.target();
        if !expected.matches_action(action) {
            return Err(generator_error(
                spec,
                format!(
                    "teacher selected action kind {:?}; expected {}",
                    action.kind(),
                    expected.name()
                ),
            ));
        }
        CompositeActionContractValidator::default_strict()
            .validate(
                action,
                subject.about(),
                subject.mode(),
                subject.visible_state(),
            )
            .map_err(|violations| generator_error(spec, format!("{violations:?}")))
    }
}

impl<T: TeacherPolicy> SyntheticCaseGenerator for TeacherBackedSyntheticCaseGenerator<T> {
    fn generate(
        &self,
        spec: &SyntheticCaseSpec,
    ) -> Result<Vec<TrainingTrajectory>, GenerateSyntheticCaseError> {
        let count = spec.minimum_examples().as_usize();
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let subject = Self::build_subject(spec, index)?;
            let action = self.teacher.decide(&subject).map_err(|err| {
                generator_error(spec, format!("teacher policy failed at row {index}: {err}"))
            })?;
            Self::validate_teacher_action(spec, &subject, action.action())?;
            out.push(Self::build_trajectory(
                spec,
                index,
                &subject,
                action.into_action(),
            )?);
        }
        Ok(out)
    }
}

fn about_for(
    target: SyntheticGenerationTarget,
    index: usize,
) -> Result<AboutId, GenerateSyntheticCaseError> {
    AboutId::parse(format!("teacher:realistic-v7:{}:{index:04}", target.name())).map_err(|err| {
        GenerateSyntheticCaseError::Generator {
            adapter: ADAPTER,
            case_id: format!("operator_action:{}", target.name()),
            message: err.to_string(),
        }
    })
}

fn goal_for(
    target: SyntheticGenerationTarget,
) -> Result<TrajectoryGoal, GenerateSyntheticCaseError> {
    let goal = match target {
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Ingest) => {
            "Execute the typed prepared kernel_ingest action exactly."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Wake) => {
            "Use kernel_wake to load the current about before navigation."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Ask) => {
            "Use kernel_ask to retrieve deterministic evidence for the current objective."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Near) => {
            "Use kernel_near around the visible anchor in the listed dimension with limit=4."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Goto) => {
            "Use kernel_goto to jump directly to the visible target ref."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Rewind) => {
            "Use kernel_rewind on the active temporal cursor with window=2."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Forward) => {
            "Use kernel_forward on the active temporal cursor with window=2."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Trace) => {
            "Use kernel_trace from the stale hypothesis to the final fix with page=8."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::Inspect) => {
            "Use kernel_inspect to read the visible decisive evidence ref."
        }
        SyntheticGenerationTarget::Kmp(KmpMcpCapability::WriteMemory) => {
            "Execute the typed prepared kernel_write_memory action exactly."
        }
        SyntheticGenerationTarget::Stop => {
            "Stop with answer_ready because the visible evidence is already sufficient."
        }
        SyntheticGenerationTarget::Escalate => {
            "Escalate with beyond_capability because the next decision needs a larger reasoner."
        }
    };
    TrajectoryGoal::parse(goal).map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("operator_action:{}", target.name()),
        message: err.to_string(),
    })
}

fn subject_state_for(
    target: SyntheticGenerationTarget,
    about: &AboutId,
    index: usize,
) -> Result<(VisibleState, Option<PreparedOperatorAction>), GenerateSyntheticCaseError> {
    let budget = BudgetSnapshot::bounded(4, 2000);
    let decision = memory_ref(target, index, "decision")?;
    let evidence = memory_ref(target, index, "evidence")?;
    let dimension = dimension_ref(target, index)?;
    match target {
        SyntheticGenerationTarget::Stop => Ok((
            VisibleState::assemble(
                [evidence],
                [dimension],
                None,
                BudgetSnapshot::bounded(0, 600),
            ),
            None,
        )),
        SyntheticGenerationTarget::Escalate => Ok((
            VisibleState::assemble([], [dimension], None, BudgetSnapshot::bounded(1, 800)),
            None,
        )),
        SyntheticGenerationTarget::Kmp(capability) => subject_state_for_kmp(
            capability, about, index, budget, decision, evidence, dimension,
        ),
    }
}

fn subject_state_for_kmp(
    capability: KmpMcpCapability,
    about: &AboutId,
    index: usize,
    budget: BudgetSnapshot,
    decision: MemoryRef,
    evidence: MemoryRef,
    dimension: DimensionRef,
) -> Result<(VisibleState, Option<PreparedOperatorAction>), GenerateSyntheticCaseError> {
    match capability {
        KmpMcpCapability::Ingest => Ok((
            VisibleState::assemble(
                [evidence.clone()],
                [dimension.clone()],
                None,
                BudgetSnapshot::bounded(2, 1400),
            ),
            Some(PreparedOperatorAction::new(prepared_ingest(
                about.clone(),
                dimension,
                &evidence,
                index,
            )?)?),
        )),
        KmpMcpCapability::Wake | KmpMcpCapability::Ask => {
            Ok((VisibleState::assemble([], [], None, budget), None))
        }
        KmpMcpCapability::Near | KmpMcpCapability::Goto | KmpMcpCapability::Inspect => Ok((
            VisibleState::assemble([decision], [dimension], None, budget),
            None,
        )),
        KmpMcpCapability::Rewind | KmpMcpCapability::Forward => Ok((
            VisibleState::assemble(
                [decision],
                [dimension],
                Some(Cursor::Temporal(temporal_cursor(index)?)),
                budget,
            ),
            None,
        )),
        KmpMcpCapability::Trace => Ok((
            VisibleState::assemble([evidence, decision], [dimension], None, budget),
            None,
        )),
        KmpMcpCapability::WriteMemory => Ok((
            VisibleState::assemble(
                [evidence.clone()],
                [dimension],
                None,
                BudgetSnapshot::bounded(2, 1200),
            ),
            Some(PreparedOperatorAction::new(prepared_write_memory(
                evidence,
            )?)?),
        )),
    }
}

fn prepared_ingest(
    about: AboutId,
    dimension: DimensionRef,
    evidence: &MemoryRef,
    index: usize,
) -> Result<OperatorAction, GenerateSyntheticCaseError> {
    let new_entry = memory_ref(
        SyntheticGenerationTarget::from(KmpMcpCapability::Ingest),
        index,
        "new-entry",
    )?;
    let coordinate = prepared_ingest_coordinate(&about, &dimension, index)?;
    let memory = prepared_ingest_memory(dimension, new_entry, coordinate)?;
    let key = prepared_ingest_key(&about, evidence)?;
    Ok(OperatorAction::ToolCall(ToolCallAction::new(
        ToolArguments::Ingest(IngestArguments::new(about, memory, None, key, true)),
    )))
}

fn prepared_ingest_coordinate(
    about: &AboutId,
    dimension: &DimensionRef,
    index: usize,
) -> Result<IngestTemporalCoordinate, GenerateSyntheticCaseError> {
    IngestTemporalCoordinate::new(
        dimension.clone(),
        ingest_non_empty(about.as_str(), "scope")?,
        None,
        None,
        None,
        None,
        None,
        Some(ingest_sequence(index)?),
        None,
        StringMap::empty(),
    )
    .map_err(|err| ingest_error(err.to_string()))
}

fn prepared_ingest_memory(
    dimension: DimensionRef,
    new_entry: MemoryRef,
    coordinate: IngestTemporalCoordinate,
) -> Result<IngestMemory, GenerateSyntheticCaseError> {
    IngestMemory::new(
        vec![prepared_ingest_dimension(dimension)?],
        vec![prepared_ingest_entry(new_entry, coordinate)?],
        vec![],
        vec![],
    )
    .map_err(|err| ingest_error(err.to_string()))
}

fn prepared_ingest_dimension(
    dimension: DimensionRef,
) -> Result<IngestDimension, GenerateSyntheticCaseError> {
    Ok(IngestDimension::new(
        dimension,
        ingest_non_empty("agent", "kind")?,
        Some(ingest_non_empty("Teacher Writer", "title")?),
        StringMap::empty(),
    ))
}

fn prepared_ingest_entry(
    new_entry: MemoryRef,
    coordinate: IngestTemporalCoordinate,
) -> Result<IngestEntry, GenerateSyntheticCaseError> {
    IngestEntry::new(
        new_entry,
        ingest_non_empty("observation", "kind")?,
        ingest_non_empty(
            "Teacher-backed prepared ingest entry with visible proof.",
            "text",
        )?,
        vec![coordinate],
        StringMap::empty(),
    )
    .map_err(|err| ingest_error(err.to_string()))
}

fn prepared_ingest_key(
    about: &AboutId,
    evidence: &MemoryRef,
) -> Result<NonEmptyString, GenerateSyntheticCaseError> {
    ingest_non_empty(
        format!(
            "teacher-backed-ingest:{}:{}",
            about.as_str(),
            evidence.as_str()
        ),
        "idempotency_key",
    )
}

fn ingest_sequence(index: usize) -> Result<PositiveCount, GenerateSyntheticCaseError> {
    PositiveCount::parse(index + 1, "sequence").map_err(|err| ingest_error(err.to_string()))
}

fn ingest_non_empty(
    value: impl Into<String>,
    field: &'static str,
) -> Result<NonEmptyString, GenerateSyntheticCaseError> {
    NonEmptyString::parse(value, field).map_err(|err| ingest_error(err.to_string()))
}

fn ingest_error(message: String) -> GenerateSyntheticCaseError {
    GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: "kmp_mcp:ingest".to_string(),
        message,
    }
}

fn prepared_write_memory(target: MemoryRef) -> Result<OperatorAction, GenerateSyntheticCaseError> {
    Ok(OperatorAction::ToolCall(ToolCallAction::new(
        ToolArguments::WriteMemory(
            WriteMemoryArguments::new(
                "Record teacher-backed memory.",
                "The teacher-backed generator selected the prepared write after visible proof.",
                vec![target],
            )
            .map_err(|err| GenerateSyntheticCaseError::Generator {
                adapter: ADAPTER,
                case_id: "kmp_mcp:write_memory".to_string(),
                message: err.to_string(),
            })?,
        ),
    )))
}

fn temporal_cursor(index: usize) -> Result<TemporalCursor, GenerateSyntheticCaseError> {
    Ok(TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse(format!("2026-05-22T00:{index:02}:00Z")).map_err(|err| {
            GenerateSyntheticCaseError::Generator {
                adapter: ADAPTER,
                case_id: "temporal_cursor".to_string(),
                message: err.to_string(),
            }
        })?,
    ))
}

fn memory_ref(
    target: SyntheticGenerationTarget,
    index: usize,
    suffix: &str,
) -> Result<MemoryRef, GenerateSyntheticCaseError> {
    MemoryRef::parse(format!(
        "teacher:realistic-v7:{}:{index:04}:node:{suffix}",
        target.name()
    ))
    .map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("operator_action:{}", target.name()),
        message: err.to_string(),
    })
}

fn dimension_ref(
    target: SyntheticGenerationTarget,
    index: usize,
) -> Result<DimensionRef, GenerateSyntheticCaseError> {
    DimensionRef::parse(format!(
        "teacher:realistic-v7:{}:{index:04}:agent:operator",
        target.name()
    ))
    .map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("operator_action:{}", target.name()),
        message: err.to_string(),
    })
}

fn generator_error(spec: &SyntheticCaseSpec, message: String) -> GenerateSyntheticCaseError {
    GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: spec.case_id().as_str().to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::escalate_action::EscalateAction;
    use operator_shared_domain::action::escalate_reason::EscalateReason;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::cursor::ref_cursor::RefCursor;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
    use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
    use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
    use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
    use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
    use operator_shared_domain::value_objects::finish_reason::FinishReason;
    use operator_shared_domain::value_objects::model_id::ModelId;
    use operator_shared_domain::value_objects::subject_hash::SubjectHash;
    use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
    use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;
    use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;

    #[derive(Debug)]
    struct ValidTeacher;

    impl TeacherPolicy for ValidTeacher {
        fn decide(
            &self,
            subject: &CalibrationSubject,
        ) -> Result<TeacherDecision, TeacherPolicyError> {
            if let Some(prepared) = subject.prepared_action() {
                return Ok(decision(prepared.action().clone()));
            }
            let family = subject.task_family().as_str();
            let capability = family.rsplit('.').next().unwrap_or("");
            match capability {
                "wake" => Ok(decision(tool(ToolArguments::Wake(WakeArguments::new(
                    subject.about().clone(),
                ))))),
                "ask" => Ok(decision(tool(ToolArguments::Ask(
                    AskArguments::new("What evidence is visible for this objective?").unwrap(),
                )))),
                "near" => Ok(decision(tool(ToolArguments::Near(
                    NearArguments::new(
                        first_ref(subject),
                        vec![first_dimension(subject)],
                        Some(PositiveCount::parse(4, "limit").unwrap()),
                    )
                    .unwrap(),
                )))),
                "goto" => Ok(decision(tool(ToolArguments::Goto(GotoArguments::new(
                    Cursor::Ref(RefCursor::new(first_ref(subject))),
                ))))),
                "rewind" => {
                    let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap()
                    else {
                        panic!("expected temporal cursor")
                    };
                    Ok(decision(tool(ToolArguments::Rewind(RewindArguments::new(
                        cursor.clone(),
                        PositiveCount::parse(2, "window").unwrap(),
                    )))))
                }
                "forward" => {
                    let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap()
                    else {
                        panic!("expected temporal cursor")
                    };
                    Ok(decision(tool(ToolArguments::Forward(
                        ForwardArguments::new(
                            cursor.clone(),
                            PositiveCount::parse(2, "window").unwrap(),
                        ),
                    ))))
                }
                "trace" => Ok(decision(tool(ToolArguments::Trace(TraceArguments::new(
                    first_ref(subject),
                    Some(second_ref(subject)),
                    PositiveCount::parse(8, "page").unwrap(),
                ))))),
                "inspect" => Ok(decision(tool(ToolArguments::Inspect(
                    InspectArguments::new(first_ref(subject)),
                )))),
                "stop" => Ok(decision(OperatorAction::Stop(
                    StopAction::new(
                        StopReason::AnswerReady,
                        Some("The visible evidence is sufficient.".to_string()),
                        vec![first_ref(subject)],
                    )
                    .unwrap(),
                ))),
                "escalate" => Ok(decision(OperatorAction::Escalate(EscalateAction::new(
                    EscalateReason::BeyondCapability,
                    ModelId::parse("frontier-reasoner").unwrap(),
                )))),
                _ => Err(TeacherPolicyError::Shape {
                    adapter: "valid_teacher",
                    message: "unsupported task family".to_string(),
                    finish_reason: Some(FinishReason::Stop),
                }),
            }
        }
    }

    #[derive(Debug)]
    struct WrongCapabilityTeacher;

    impl TeacherPolicy for WrongCapabilityTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<TeacherDecision, TeacherPolicyError> {
            Ok(decision(OperatorAction::Escalate(EscalateAction::new(
                EscalateReason::BeyondCapability,
                ModelId::parse("frontier-reasoner").unwrap(),
            ))))
        }
    }

    #[derive(Debug)]
    struct InvalidContractTeacher;

    impl TeacherPolicy for InvalidContractTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<TeacherDecision, TeacherPolicyError> {
            Ok(decision(tool(ToolArguments::Inspect(
                InspectArguments::new(MemoryRef::parse("unknown:node").unwrap()),
            ))))
        }
    }

    #[derive(Debug)]
    struct FailingTeacher;

    impl TeacherPolicy for FailingTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<TeacherDecision, TeacherPolicyError> {
            Err(TeacherPolicyError::Shape {
                adapter: "failing_teacher",
                message: "bad shape".to_string(),
                finish_reason: Some(FinishReason::Stop),
            })
        }
    }

    fn decision(action: OperatorAction) -> TeacherDecision {
        TeacherDecision::new(action, FinishReason::Stop, subject_hash())
    }

    fn subject_hash() -> SubjectHash {
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap()
    }

    #[test]
    fn valid_teacher_generates_contract_valid_rows_for_every_generation_target() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(ValidTeacher);
        for target in SyntheticGenerationTarget::ALL {
            let rows = generator.generate(&spec(target, 2)).unwrap();
            assert_eq!(rows.len(), 2);
            for row in rows {
                assert!(target.matches_action(row.target_action()));
                CompositeActionContractValidator::default_strict()
                    .validate(
                        row.target_action(),
                        row.about(),
                        row.mode(),
                        row.visible_state(),
                    )
                    .unwrap();
            }
        }
    }

    #[test]
    fn rejects_teacher_action_for_wrong_capability() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(WrongCapabilityTeacher);
        let err = generator
            .generate(&spec(
                SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
                1,
            ))
            .unwrap_err();
        assert!(format!("{err}").contains("expected inspect"));
    }

    #[test]
    fn rejects_teacher_action_that_violates_the_strict_contract() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(InvalidContractTeacher);
        let err = generator
            .generate(&spec(
                SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
                1,
            ))
            .unwrap_err();
        assert!(format!("{err}").contains("unknown"));
    }

    #[test]
    fn propagates_teacher_policy_failures_without_repair() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(FailingTeacher);
        let err = generator
            .generate(&spec(
                SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
                1,
            ))
            .unwrap_err();
        assert!(format!("{err}").contains("teacher policy failed"));
        assert!(format!("{err}").contains("bad shape"));
    }

    fn spec(target: SyntheticGenerationTarget, minimum: usize) -> SyntheticCaseSpec {
        SyntheticCaseSpec::for_target(
            operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId::parse(format!(
                "case:{}",
                target.name()
            ))
            .unwrap(),
            target,
            PositiveCount::parse(minimum, "minimum").unwrap(),
        )
    }

    fn tool(arguments: ToolArguments) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(arguments))
    }

    fn first_ref(subject: &CalibrationSubject) -> MemoryRef {
        subject
            .visible_state()
            .known_refs()
            .iter()
            .next()
            .cloned()
            .unwrap()
    }

    fn second_ref(subject: &CalibrationSubject) -> MemoryRef {
        subject
            .visible_state()
            .known_refs()
            .iter()
            .nth(1)
            .cloned()
            .unwrap()
    }

    fn first_dimension(subject: &CalibrationSubject) -> DimensionRef {
        subject
            .visible_state()
            .known_dimensions()
            .iter()
            .next()
            .cloned()
            .unwrap()
    }

    #[test]
    fn prepared_action_errors_remain_domain_errors() {
        let err = PreparedOperatorAction::new(OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::BeyondCapability,
            ModelId::parse("frontier-reasoner").unwrap(),
        )))
        .unwrap_err();
        assert!(matches!(
            err,
            SyntheticDomainError::PreparedActionMustBeToolCall { .. }
        ));
    }
}
