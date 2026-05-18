//! Specification: when the action depends on the active cursor (Rewind /
//! Forward navigation, or implicit chaining), the visible state must have
//! an active cursor of a compatible kind.
//!
//! For the first pass we model only the most common rule: Rewind and
//! Forward require the visible state to expose an active `Temporal`
//! cursor whose key matches the request.

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::cursor::cursor::Cursor;
use crate::specifications::specification::Specification;
use crate::tool_arguments::tool_arguments::ToolArguments;

#[derive(Debug, Default)]
pub struct CursorReachableFromVisibleSpec;

impl CursorReachableFromVisibleSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for CursorReachableFromVisibleSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        let required_key = match call.arguments() {
            ToolArguments::Rewind(args) => args.cursor().key(),
            ToolArguments::Forward(args) => args.cursor().key(),
            _ => return Ok(()),
        };
        let active = subject.visible().active_cursor();
        match active {
            Some(Cursor::Temporal(active)) if active.key() == required_key => Ok(()),
            _ => Err(ContractViolation::new(
                ContractViolationCode::CursorAnchorMissing,
                "visible_state.active_cursor",
                format!(
                    "temporal cursor with key '{}' must be the active cursor",
                    required_key.as_str()
                ),
            )),
        }
    }
}
