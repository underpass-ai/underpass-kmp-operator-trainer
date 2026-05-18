//! What actually happened when the replay use case attempted to
//! execute a predicted action against KMP.
//!
//! - `ToolSucceeded(ToolOutcome)`: the adapter returned a typed
//!   per-tool outcome.
//! - `NoToolCall`: the predicted action was `Stop` or `Escalate`, so
//!   no KMP call was attempted.
//! - `ToolCallFailed { tool, reason }`: the adapter returned an error;
//!   the originally-predicted tool is recorded so per-tool stats can
//!   distinguish "Inspect failed" from "Wake failed".

use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;

use crate::outcome::replay_failure_reason::ReplayFailureReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayOutcome {
    ToolSucceeded(ToolOutcome),
    NoToolCall,
    ToolCallFailed {
        tool: KernelTool,
        reason: ReplayFailureReason,
    },
}

impl ReplayOutcome {
    pub fn is_tool_call_success(&self) -> bool {
        matches!(self, Self::ToolSucceeded(_))
    }

    pub fn is_tool_call_failure(&self) -> bool {
        matches!(self, Self::ToolCallFailed { .. })
    }

    pub fn is_no_tool_call(&self) -> bool {
        matches!(self, Self::NoToolCall)
    }

    /// `KernelTool` associated with this outcome, or `None` when no
    /// tool call was attempted (Stop / Escalate). The full predicted
    /// action — including which non-tool-call variant — is available
    /// on `ReplayExecution::predicted_action()` if the caller needs
    /// it.
    pub fn tool(&self) -> Option<KernelTool> {
        match self {
            Self::ToolSucceeded(outcome) => Some(outcome.tool()),
            Self::ToolCallFailed { tool, .. } => Some(*tool),
            Self::NoToolCall => None,
        }
    }
}
