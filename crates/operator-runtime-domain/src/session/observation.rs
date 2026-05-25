use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;

use crate::session::observation_error_code::ObservationErrorCode;
use crate::session::terminal_reason::TerminalReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    ToolResponse {
        outcome: ToolOutcome,
        observed_refs: Vec<MemoryRef>,
    },
    ToolError {
        code: ObservationErrorCode,
        message: String,
    },
    Terminal {
        reason: TerminalReason,
    },
}

impl Observation {
    pub fn is_tool_error(&self) -> bool {
        matches!(self, Self::ToolError { .. })
    }
}
