//! Specification: every `MemoryRef` and `DimensionRef` referenced by the
//! action's arguments must be present in the visible state. References to
//! unknown entities mean the operator is hallucinating evidence.

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::cursor::cursor::Cursor;
use crate::specifications::specification::Specification;
use crate::tool_arguments::tool_arguments::ToolArguments;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::visible_state::visible_state::VisibleState;

#[derive(Debug, Default)]
pub struct ArgumentsReferenceKnownEntitiesSpec;

impl ArgumentsReferenceKnownEntitiesSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for ArgumentsReferenceKnownEntitiesSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        let visible = subject.visible();
        match call.arguments() {
            ToolArguments::Wake(_)
            | ToolArguments::Ask(_)
            | ToolArguments::Rewind(_)
            | ToolArguments::Forward(_)
            | ToolArguments::WriteMemory(_) => Ok(()),
            ToolArguments::Near(args) => {
                check_ref(visible, args.anchor(), "near.anchor")?;
                check_dimensions(visible, args.dimensions(), "near.dimensions")
            }
            ToolArguments::Goto(args) => check_cursor(visible, args.cursor(), "goto.cursor"),
            ToolArguments::Trace(args) => {
                check_ref(visible, args.from(), "trace.from")?;
                if let Some(target) = args.to() {
                    check_ref(visible, target, "trace.to")?;
                }
                Ok(())
            }
            ToolArguments::Inspect(args) => check_ref(visible, args.target(), "inspect.target"),
        }
    }
}

fn check_ref(
    visible: &VisibleState,
    target: &MemoryRef,
    field: &str,
) -> Result<(), ContractViolation> {
    if visible.knows_ref(target) {
        return Ok(());
    }
    Err(ContractViolation::new(
        ContractViolationCode::UnknownMemoryRef,
        field.to_string(),
        format!("memory ref '{target}' is not in visible state"),
    ))
}

fn check_dimensions(
    visible: &VisibleState,
    dimensions: &[DimensionRef],
    field: &str,
) -> Result<(), ContractViolation> {
    for dimension in dimensions {
        if !visible.knows_dimension(dimension) {
            return Err(ContractViolation::new(
                ContractViolationCode::UnknownDimension,
                field.to_string(),
                format!("dimension '{dimension}' is not in visible state"),
            ));
        }
    }
    Ok(())
}

fn check_cursor(
    visible: &VisibleState,
    cursor: &Cursor,
    field: &str,
) -> Result<(), ContractViolation> {
    match cursor {
        Cursor::Ref(rc) => check_ref(visible, rc.target(), field),
        Cursor::Around(ac) => {
            check_ref(visible, ac.anchor(), field)?;
            check_dimensions(visible, ac.dimensions(), field)
        }
        Cursor::Temporal(_) => Ok(()),
        Cursor::Trace(tc) => {
            check_ref(visible, tc.from(), field)?;
            check_ref(visible, tc.to(), field)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::stop_action::StopAction;
    use crate::action::stop_reason::StopReason;
    use crate::action::tool_call_action::ToolCallAction;
    use crate::cursor::around_cursor::AroundCursor;
    use crate::cursor::ref_cursor::RefCursor;
    use crate::cursor::temporal_anchor::TemporalAnchor;
    use crate::cursor::temporal_cursor::TemporalCursor;
    use crate::cursor::temporal_cursor_key::TemporalCursorKey;
    use crate::cursor::trace_cursor::TraceCursor;
    use crate::ids::about_id::AboutId;
    use crate::mode::operator_mode::OperatorMode;
    use crate::tool_arguments::ask_arguments::AskArguments;
    use crate::tool_arguments::forward_arguments::ForwardArguments;
    use crate::tool_arguments::goto_arguments::GotoArguments;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::near_arguments::NearArguments;
    use crate::tool_arguments::rewind_arguments::RewindArguments;
    use crate::tool_arguments::trace_arguments::TraceArguments;
    use crate::tool_arguments::wake_arguments::WakeArguments;
    use crate::tool_arguments::write_memory_arguments::WriteMemoryArguments;
    use crate::value_objects::positive_count::PositiveCount;
    use crate::visible_state::visible_state_builder::VisibleStateBuilder;

    fn known_node() -> MemoryRef {
        MemoryRef::parse("node:known").unwrap()
    }

    fn known_dim() -> DimensionRef {
        DimensionRef::parse("temporal").unwrap()
    }

    fn visible_with_known() -> VisibleState {
        VisibleStateBuilder::new()
            .with_known_ref(known_node())
            .with_known_dimension(known_dim())
            .build()
    }

    fn evaluate(action: &OperatorAction, visible: &VisibleState) -> Result<(), ContractViolation> {
        let subject = ActionContractSubject::new(action, OperatorMode::Read, visible);
        ArgumentsReferenceKnownEntitiesSpec::new().evaluate(&subject)
    }

    #[test]
    fn wake_and_ask_are_passthrough() {
        let visible = visible_with_known();
        let wake = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
            WakeArguments::new(AboutId::parse("about:1").unwrap()),
        )));
        let ask = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
            AskArguments::new("q").unwrap(),
        )));
        assert!(evaluate(&wake, &visible).is_ok());
        assert!(evaluate(&ask, &visible).is_ok());
    }

    #[test]
    fn rewind_forward_and_write_memory_are_passthrough() {
        let visible = visible_with_known();
        let rewind = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Rewind(
            RewindArguments::new(
                TemporalCursor::new(
                    TemporalCursorKey::Created,
                    TemporalAnchor::parse("t").unwrap(),
                ),
                PositiveCount::parse(1, "w").unwrap(),
            ),
        )));
        let forward = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
            ForwardArguments::new(
                TemporalCursor::new(
                    TemporalCursorKey::Created,
                    TemporalAnchor::parse("t").unwrap(),
                ),
                PositiveCount::parse(1, "w").unwrap(),
            ),
        )));
        let write = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
            WriteMemoryArguments::new("s", "b", vec![]).unwrap(),
        )));
        assert!(evaluate(&rewind, &visible).is_ok());
        assert!(evaluate(&forward, &visible).is_ok());
        assert!(evaluate(&write, &visible).is_ok());
    }

    #[test]
    fn stop_action_is_ignored() {
        let visible = visible_with_known();
        let stop =
            OperatorAction::Stop(StopAction::new(StopReason::NoCandidate, None, vec![]).unwrap());
        assert!(evaluate(&stop, &visible).is_ok());
    }

    #[test]
    fn near_accepts_known_anchor_and_dimensions() {
        let visible = visible_with_known();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
            NearArguments::new(known_node(), vec![known_dim()], None).unwrap(),
        )));
        assert!(evaluate(&action, &visible).is_ok());
    }

    #[test]
    fn near_rejects_unknown_anchor() {
        let visible = visible_with_known();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
            NearArguments::new(
                MemoryRef::parse("node:absent").unwrap(),
                vec![known_dim()],
                None,
            )
            .unwrap(),
        )));
        let err = evaluate(&action, &visible).unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::UnknownMemoryRef);
    }

    #[test]
    fn near_rejects_unknown_dimension() {
        let visible = visible_with_known();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
            NearArguments::new(
                known_node(),
                vec![DimensionRef::parse("semantic").unwrap()],
                None,
            )
            .unwrap(),
        )));
        let err = evaluate(&action, &visible).unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::UnknownDimension);
    }

    #[test]
    fn goto_ref_resolution_paths() {
        let visible = visible_with_known();
        let ok = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Ref(RefCursor::new(known_node()))),
        )));
        assert!(evaluate(&ok, &visible).is_ok());

        let absent = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Ref(RefCursor::new(
                MemoryRef::parse("node:absent").unwrap(),
            ))),
        )));
        assert_eq!(
            evaluate(&absent, &visible).unwrap_err().code(),
            ContractViolationCode::UnknownMemoryRef
        );
    }

    #[test]
    fn goto_around_paths() {
        let visible = visible_with_known();
        let ok = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Around(
                AroundCursor::new(known_node(), vec![known_dim()]).unwrap(),
            )),
        )));
        assert!(evaluate(&ok, &visible).is_ok());

        let bad_dim = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Around(
                AroundCursor::new(known_node(), vec![DimensionRef::parse("semantic").unwrap()])
                    .unwrap(),
            )),
        )));
        assert_eq!(
            evaluate(&bad_dim, &visible).unwrap_err().code(),
            ContractViolationCode::UnknownDimension
        );
    }

    #[test]
    fn goto_trace_paths() {
        let visible = VisibleStateBuilder::new()
            .with_known_ref(MemoryRef::parse("node:a").unwrap())
            .with_known_ref(MemoryRef::parse("node:b").unwrap())
            .build();
        let ok = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Trace(TraceCursor::new(
                MemoryRef::parse("node:a").unwrap(),
                MemoryRef::parse("node:b").unwrap(),
            ))),
        )));
        assert!(evaluate(&ok, &visible).is_ok());

        let bad = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Trace(TraceCursor::new(
                MemoryRef::parse("node:a").unwrap(),
                MemoryRef::parse("node:absent").unwrap(),
            ))),
        )));
        assert_eq!(
            evaluate(&bad, &visible).unwrap_err().code(),
            ContractViolationCode::UnknownMemoryRef
        );
    }

    #[test]
    fn goto_temporal_cursor_is_passthrough() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Temporal(TemporalCursor::new(
                TemporalCursorKey::Updated,
                TemporalAnchor::parse("t").unwrap(),
            ))),
        )));
        assert!(evaluate(&action, &visible).is_ok());
    }

    #[test]
    fn trace_with_and_without_to() {
        let visible = visible_with_known();
        let with_to = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
            TraceArguments::new(
                known_node(),
                Some(known_node()),
                PositiveCount::parse(1, "p").unwrap(),
            ),
        )));
        assert!(evaluate(&with_to, &visible).is_ok());

        let without_to = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
            TraceArguments::new(known_node(), None, PositiveCount::parse(1, "p").unwrap()),
        )));
        assert!(evaluate(&without_to, &visible).is_ok());

        let bad_to = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
            TraceArguments::new(
                known_node(),
                Some(MemoryRef::parse("node:absent").unwrap()),
                PositiveCount::parse(1, "p").unwrap(),
            ),
        )));
        assert_eq!(
            evaluate(&bad_to, &visible).unwrap_err().code(),
            ContractViolationCode::UnknownMemoryRef
        );
    }

    #[test]
    fn inspect_unknown_target_is_rejected() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:absent").unwrap()),
        )));
        assert_eq!(
            evaluate(&action, &visible).unwrap_err().code(),
            ContractViolationCode::UnknownMemoryRef
        );
    }
}
