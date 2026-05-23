//! Semantic mismatch detected after the coarse generation target matched.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::cursor::cursor_kind::CursorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticAcceptanceViolation {
    StopReason {
        expected: StopReason,
        actual: Option<StopReason>,
    },
    CursorKind {
        expected: CursorKind,
        actual: Option<CursorKind>,
    },
}

impl SemanticAcceptanceViolation {
    pub fn field(&self) -> &'static str {
        match self {
            Self::StopReason { .. } => "stop.reason",
            Self::CursorKind { .. } => "goto.cursor.kind",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::StopReason { expected, actual } => format!(
                "semantic mismatch at stop.reason: expected {}, got {}",
                expected.as_str(),
                actual.map_or("none", StopReason::as_str)
            ),
            Self::CursorKind { expected, actual } => format!(
                "semantic mismatch at goto.cursor.kind: expected {}, got {}",
                expected.as_str(),
                actual.map_or("none", CursorKind::as_str)
            ),
        }
    }
}
