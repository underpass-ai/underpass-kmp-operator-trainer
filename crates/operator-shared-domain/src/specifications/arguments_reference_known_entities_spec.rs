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
