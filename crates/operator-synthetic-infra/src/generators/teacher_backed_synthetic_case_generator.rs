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
        let capability = spec.capability();
        let mode = capability.mode();
        let about = about_for(capability, index)?;
        let goal = goal_for(capability)?;
        let task_family = TaskFamily::parse(format!("teacher_backed.{}", capability.name()))
            .map_err(|err| generator_error(spec, err.to_string()))?;
        let (visible_state, prepared_action) = subject_state_for(capability, &about, index)?;
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
            spec.capability().name()
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
        let expected = spec.capability().tool();
        let actual = action.tool();
        if actual != Some(expected) {
            return Err(generator_error(
                spec,
                format!(
                    "teacher selected capability {:?}; expected {}",
                    actual.map(operator_shared_domain::tool::kernel_tool::KernelTool::as_str),
                    expected.as_str()
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
            Self::validate_teacher_action(spec, &subject, &action)?;
            out.push(Self::build_trajectory(spec, index, &subject, action)?);
        }
        Ok(out)
    }
}

fn about_for(
    capability: KmpMcpCapability,
    index: usize,
) -> Result<AboutId, GenerateSyntheticCaseError> {
    AboutId::parse(format!(
        "teacher:realistic-v7:{}:{index:04}",
        capability.name()
    ))
    .map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("kmp_mcp:{}", capability.name()),
        message: err.to_string(),
    })
}

fn goal_for(capability: KmpMcpCapability) -> Result<TrajectoryGoal, GenerateSyntheticCaseError> {
    let goal = match capability {
        KmpMcpCapability::Ingest => "Execute the typed prepared kernel_ingest action exactly.",
        KmpMcpCapability::Wake => "Use kernel_wake to load the current about before navigation.",
        KmpMcpCapability::Ask => {
            "Use kernel_ask to retrieve deterministic evidence for the current objective."
        }
        KmpMcpCapability::Near => {
            "Use kernel_near around the visible anchor in the listed dimension with limit=4."
        }
        KmpMcpCapability::Goto => "Use kernel_goto to jump directly to the visible target ref.",
        KmpMcpCapability::Rewind => {
            "Use kernel_rewind on the active temporal cursor with window=2."
        }
        KmpMcpCapability::Forward => {
            "Use kernel_forward on the active temporal cursor with window=2."
        }
        KmpMcpCapability::Trace => {
            "Use kernel_trace from the stale hypothesis to the final fix with page=8."
        }
        KmpMcpCapability::Inspect => {
            "Use kernel_inspect to read the visible decisive evidence ref."
        }
        KmpMcpCapability::WriteMemory => {
            "Execute the typed prepared kernel_write_memory action exactly."
        }
    };
    TrajectoryGoal::parse(goal).map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("kmp_mcp:{}", capability.name()),
        message: err.to_string(),
    })
}

fn subject_state_for(
    capability: KmpMcpCapability,
    about: &AboutId,
    index: usize,
) -> Result<(VisibleState, Option<PreparedOperatorAction>), GenerateSyntheticCaseError> {
    let budget = BudgetSnapshot::bounded(4, 2000);
    let decision = memory_ref(capability, index, "decision")?;
    let evidence = memory_ref(capability, index, "evidence")?;
    let dimension = dimension_ref(capability, index)?;
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
    let new_entry = memory_ref(KmpMcpCapability::Ingest, index, "new-entry")?;
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
    capability: KmpMcpCapability,
    index: usize,
    suffix: &str,
) -> Result<MemoryRef, GenerateSyntheticCaseError> {
    MemoryRef::parse(format!(
        "teacher:realistic-v7:{}:{index:04}:node:{suffix}",
        capability.name()
    ))
    .map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("kmp_mcp:{}", capability.name()),
        message: err.to_string(),
    })
}

fn dimension_ref(
    capability: KmpMcpCapability,
    index: usize,
) -> Result<DimensionRef, GenerateSyntheticCaseError> {
    DimensionRef::parse(format!(
        "teacher:realistic-v7:{}:{index:04}:agent:operator",
        capability.name()
    ))
    .map_err(|err| GenerateSyntheticCaseError::Generator {
        adapter: ADAPTER,
        case_id: format!("kmp_mcp:{}", capability.name()),
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
    use operator_shared_domain::cursor::ref_cursor::RefCursor;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
    use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
    use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
    use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
    use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
    use operator_shared_domain::value_objects::model_id::ModelId;
    use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;
    use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;

    #[derive(Debug)]
    struct ValidTeacher;

    impl TeacherPolicy for ValidTeacher {
        fn decide(
            &self,
            subject: &CalibrationSubject,
        ) -> Result<OperatorAction, TeacherPolicyError> {
            if let Some(prepared) = subject.prepared_action() {
                return Ok(prepared.action().clone());
            }
            let family = subject.task_family().as_str();
            let capability = family.rsplit('.').next().unwrap_or("");
            match capability {
                "wake" => Ok(tool(ToolArguments::Wake(WakeArguments::new(
                    subject.about().clone(),
                )))),
                "ask" => Ok(tool(ToolArguments::Ask(
                    AskArguments::new("What evidence is visible for this objective?").unwrap(),
                ))),
                "near" => Ok(tool(ToolArguments::Near(
                    NearArguments::new(
                        first_ref(subject),
                        vec![first_dimension(subject)],
                        Some(PositiveCount::parse(4, "limit").unwrap()),
                    )
                    .unwrap(),
                ))),
                "goto" => Ok(tool(ToolArguments::Goto(GotoArguments::new(Cursor::Ref(
                    RefCursor::new(first_ref(subject)),
                ))))),
                "rewind" => {
                    let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap()
                    else {
                        panic!("expected temporal cursor")
                    };
                    Ok(tool(ToolArguments::Rewind(RewindArguments::new(
                        cursor.clone(),
                        PositiveCount::parse(2, "window").unwrap(),
                    ))))
                }
                "forward" => {
                    let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap()
                    else {
                        panic!("expected temporal cursor")
                    };
                    Ok(tool(ToolArguments::Forward(ForwardArguments::new(
                        cursor.clone(),
                        PositiveCount::parse(2, "window").unwrap(),
                    ))))
                }
                "trace" => Ok(tool(ToolArguments::Trace(TraceArguments::new(
                    first_ref(subject),
                    Some(second_ref(subject)),
                    PositiveCount::parse(8, "page").unwrap(),
                )))),
                "inspect" => Ok(tool(ToolArguments::Inspect(InspectArguments::new(
                    first_ref(subject),
                )))),
                _ => Err(TeacherPolicyError::Shape {
                    adapter: "valid_teacher",
                    message: "unsupported task family".to_string(),
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
        ) -> Result<OperatorAction, TeacherPolicyError> {
            Ok(OperatorAction::Escalate(EscalateAction::new(
                EscalateReason::BeyondCapability,
                ModelId::parse("frontier-reasoner").unwrap(),
            )))
        }
    }

    #[derive(Debug)]
    struct InvalidContractTeacher;

    impl TeacherPolicy for InvalidContractTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<OperatorAction, TeacherPolicyError> {
            Ok(tool(ToolArguments::Inspect(InspectArguments::new(
                MemoryRef::parse("unknown:node").unwrap(),
            ))))
        }
    }

    #[derive(Debug)]
    struct FailingTeacher;

    impl TeacherPolicy for FailingTeacher {
        fn decide(
            &self,
            _subject: &CalibrationSubject,
        ) -> Result<OperatorAction, TeacherPolicyError> {
            Err(TeacherPolicyError::Shape {
                adapter: "failing_teacher",
                message: "bad shape".to_string(),
            })
        }
    }

    #[test]
    fn valid_teacher_generates_contract_valid_rows_for_every_current_tool_capability() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(ValidTeacher);
        for capability in KmpMcpCapability::ALL {
            let rows = generator.generate(&spec(capability, 2)).unwrap();
            assert_eq!(rows.len(), 2);
            for row in rows {
                assert_eq!(row.target_action().tool(), Some(capability.tool()));
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
            .generate(&spec(KmpMcpCapability::Inspect, 1))
            .unwrap_err();
        assert!(format!("{err}").contains("expected kernel_inspect"));
    }

    #[test]
    fn rejects_teacher_action_that_violates_the_strict_contract() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(InvalidContractTeacher);
        let err = generator
            .generate(&spec(KmpMcpCapability::Inspect, 1))
            .unwrap_err();
        assert!(format!("{err}").contains("unknown"));
    }

    #[test]
    fn propagates_teacher_policy_failures_without_repair() {
        let generator = TeacherBackedSyntheticCaseGenerator::new(FailingTeacher);
        let err = generator
            .generate(&spec(KmpMcpCapability::Inspect, 1))
            .unwrap_err();
        assert!(format!("{err}").contains("teacher policy failed"));
        assert!(format!("{err}").contains("bad shape"));
    }

    fn spec(capability: KmpMcpCapability, minimum: usize) -> SyntheticCaseSpec {
        SyntheticCaseSpec::new(
            operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId::parse(format!(
                "case:{}",
                capability.name()
            ))
            .unwrap(),
            capability,
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
