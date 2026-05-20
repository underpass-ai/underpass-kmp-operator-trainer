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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::tool_call_action::ToolCallAction;
    use crate::cursor::ref_cursor::RefCursor;
    use crate::cursor::temporal_anchor::TemporalAnchor;
    use crate::cursor::temporal_cursor::TemporalCursor;
    use crate::cursor::temporal_cursor_key::TemporalCursorKey;
    use crate::ids::about_id::AboutId;
    use crate::mode::operator_mode::OperatorMode;
    use crate::tool_arguments::forward_arguments::ForwardArguments;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::rewind_arguments::RewindArguments;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::positive_count::PositiveCount;
    use crate::visible_state::visible_state_builder::VisibleStateBuilder;

    fn anchor() -> TemporalAnchor {
        TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap()
    }

    fn about() -> AboutId {
        AboutId::parse("about:cursor").unwrap()
    }

    fn rewind_action(key: TemporalCursorKey) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Rewind(
            RewindArguments::new(
                TemporalCursor::new(key, anchor()),
                PositiveCount::parse(1, "w").unwrap(),
            ),
        )))
    }

    fn forward_action(key: TemporalCursorKey) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
            ForwardArguments::new(
                TemporalCursor::new(key, anchor()),
                PositiveCount::parse(1, "w").unwrap(),
            ),
        )))
    }

    #[test]
    fn rewind_with_matching_active_temporal_cursor_passes() {
        let visible = VisibleStateBuilder::new()
            .with_active_cursor(Cursor::Temporal(TemporalCursor::new(
                TemporalCursorKey::Created,
                anchor(),
            )))
            .build();
        let action = rewind_action(TemporalCursorKey::Created);
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        assert!(
            CursorReachableFromVisibleSpec::new()
                .evaluate(&subject)
                .is_ok()
        );
    }

    #[test]
    fn forward_with_mismatching_key_fails() {
        let visible = VisibleStateBuilder::new()
            .with_active_cursor(Cursor::Temporal(TemporalCursor::new(
                TemporalCursorKey::Created,
                anchor(),
            )))
            .build();
        let action = forward_action(TemporalCursorKey::Accessed);
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        let err = CursorReachableFromVisibleSpec::new()
            .evaluate(&subject)
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::CursorAnchorMissing);
    }

    #[test]
    fn rewind_without_any_active_cursor_fails() {
        let visible = VisibleStateBuilder::new().build();
        let action = rewind_action(TemporalCursorKey::Created);
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        assert!(
            CursorReachableFromVisibleSpec::new()
                .evaluate(&subject)
                .is_err()
        );
    }

    #[test]
    fn rewind_with_non_temporal_active_cursor_fails() {
        let visible = VisibleStateBuilder::new()
            .with_active_cursor(Cursor::Ref(RefCursor::new(
                MemoryRef::parse("node:1").unwrap(),
            )))
            .build();
        let action = rewind_action(TemporalCursorKey::Updated);
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        assert!(
            CursorReachableFromVisibleSpec::new()
                .evaluate(&subject)
                .is_err()
        );
    }

    #[test]
    fn non_temporal_actions_are_ignored() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )));
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        assert!(
            CursorReachableFromVisibleSpec::new()
                .evaluate(&subject)
                .is_ok()
        );
    }
}
