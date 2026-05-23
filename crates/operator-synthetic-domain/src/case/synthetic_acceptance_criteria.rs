//! Template-level semantic acceptance criteria for generated scenarios.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::cursor::cursor_kind::CursorKind;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;

use crate::case::semantic_acceptance_violation::SemanticAcceptanceViolation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticAcceptanceCriteria {
    expected_stop_reason: Option<StopReason>,
    expected_cursor_kind: Option<CursorKind>,
}

impl SyntheticAcceptanceCriteria {
    pub fn new(
        expected_stop_reason: Option<StopReason>,
        expected_cursor_kind: Option<CursorKind>,
    ) -> Self {
        Self {
            expected_stop_reason,
            expected_cursor_kind,
        }
    }

    pub fn permissive() -> Self {
        Self::new(None, None)
    }

    pub fn matches(&self, action: &OperatorAction) -> Result<(), SemanticAcceptanceViolation> {
        if let Some(expected) = self.expected_stop_reason {
            let actual = stop_reason(action);
            if actual != Some(expected) {
                return Err(SemanticAcceptanceViolation::StopReason { expected, actual });
            }
        }
        if let Some(expected) = self.expected_cursor_kind {
            let actual = goto_cursor_kind(action);
            if actual != Some(expected) {
                return Err(SemanticAcceptanceViolation::CursorKind { expected, actual });
            }
        }
        Ok(())
    }

    pub fn expected_stop_reason(&self) -> Option<StopReason> {
        self.expected_stop_reason
    }

    pub fn expected_cursor_kind(&self) -> Option<CursorKind> {
        self.expected_cursor_kind
    }
}

fn stop_reason(action: &OperatorAction) -> Option<StopReason> {
    match action {
        OperatorAction::Stop(stop) => Some(stop.reason()),
        OperatorAction::ToolCall(_) | OperatorAction::Escalate(_) => None,
    }
}

fn goto_cursor_kind(action: &OperatorAction) -> Option<CursorKind> {
    match action {
        OperatorAction::ToolCall(call) => match call.arguments() {
            ToolArguments::Goto(args) => Some(args.cursor().kind()),
            _ => None,
        },
        OperatorAction::Stop(_) | OperatorAction::Escalate(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::cursor::cursor::Cursor;
    use operator_shared_domain::cursor::ref_cursor::RefCursor;
    use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
    use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
    use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
    use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;

    #[test]
    fn accepts_matching_stop_reason() {
        let criteria = SyntheticAcceptanceCriteria::new(Some(StopReason::NoCandidate), None);
        let action = OperatorAction::Stop(
            StopAction::new(StopReason::NoCandidate, None, vec![]).expect("stop builds"),
        );

        assert!(criteria.matches(&action).is_ok());
    }

    #[test]
    fn rejects_wrong_stop_reason() {
        let criteria = SyntheticAcceptanceCriteria::new(Some(StopReason::NoCandidate), None);
        let action = OperatorAction::Stop(
            StopAction::new(StopReason::AnswerReady, None, vec![]).expect("stop builds"),
        );

        let err = criteria.matches(&action).expect_err("reason mismatch");

        assert_eq!(err.field(), "stop.reason");
        assert!(err.message().contains("expected no_candidate"));
    }

    #[test]
    fn rejects_wrong_cursor_kind() {
        let criteria = SyntheticAcceptanceCriteria::new(None, Some(CursorKind::Temporal));
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Ref(RefCursor::new(
                MemoryRef::parse("node:target").expect("ref parses"),
            ))),
        )));

        let err = criteria.matches(&action).expect_err("cursor mismatch");

        assert_eq!(err.field(), "goto.cursor.kind");
        assert!(err.message().contains("expected temporal"));
    }

    #[test]
    fn accepts_matching_cursor_kind() {
        let criteria = SyntheticAcceptanceCriteria::new(None, Some(CursorKind::Temporal));
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
            GotoArguments::new(Cursor::Temporal(TemporalCursor::new(
                TemporalCursorKey::Created,
                TemporalAnchor::parse("2026-05-23T00:00:00Z").expect("anchor parses"),
            ))),
        )));

        assert!(criteria.matches(&action).is_ok());
    }

    #[test]
    fn permissive_accepts_any_action() {
        let action = OperatorAction::Stop(
            StopAction::new(StopReason::AnswerReady, None, vec![]).expect("stop builds"),
        );

        assert!(
            SyntheticAcceptanceCriteria::permissive()
                .matches(&action)
                .is_ok()
        );
    }
}
