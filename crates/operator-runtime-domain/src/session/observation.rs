use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::navigation_signals::NavigationSignals;

use crate::session::observation_error_code::ObservationErrorCode;
use crate::session::terminal_reason::TerminalReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    ToolResponse {
        outcome: ToolOutcome,
        observed_refs: Vec<MemoryRef>,
        /// Intrinsic context-coverage signals parsed from the KMP/MCP response
        /// (coverage, proof frontier, page, quality). Feed the multi-step loop's
        /// `ContextCoverageDeviation` estimator.
        signals: NavigationSignals,
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

    /// The context-coverage signals of a successful tool response, if any.
    pub fn signals(&self) -> Option<&NavigationSignals> {
        match self {
            Self::ToolResponse { signals, .. } => Some(signals),
            Self::ToolError { .. } | Self::Terminal { .. } => None,
        }
    }
}
