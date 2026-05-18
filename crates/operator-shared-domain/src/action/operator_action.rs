use crate::action::escalate_action::EscalateAction;
use crate::action::stop_action::StopAction;
use crate::action::tool_call_action::ToolCallAction;
use crate::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorAction {
    ToolCall(ToolCallAction),
    Stop(StopAction),
    Escalate(EscalateAction),
}

impl OperatorAction {
    /// Returns the `KernelTool` associated with this action, or `None` when
    /// the action is `Stop` or `Escalate`.
    pub fn tool(&self) -> Option<KernelTool> {
        match self {
            Self::ToolCall(call) => Some(call.tool()),
            Self::Stop(_) | Self::Escalate(_) => None,
        }
    }

    pub fn kind(&self) -> OperatorActionKind {
        match self {
            Self::ToolCall(_) => OperatorActionKind::ToolCall,
            Self::Stop(_) => OperatorActionKind::Stop,
            Self::Escalate(_) => OperatorActionKind::Escalate,
        }
    }

    /// True iff this action consumes one tool-call slot from the budget. The
    /// budget specification uses this instead of `tool().is_some()` to make
    /// the intent explicit at call sites.
    pub fn consumes_tool_slot(&self) -> bool {
        matches!(self, Self::ToolCall(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperatorActionKind {
    ToolCall,
    Stop,
    Escalate,
}

impl OperatorActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolCall => "tool_call",
            Self::Stop => "stop",
            Self::Escalate => "escalate",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::escalate_reason::EscalateReason;
    use crate::action::stop_reason::StopReason;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::tool_arguments::ToolArguments;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::model_id::ModelId;

    fn tool_call_action() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )))
    }

    fn stop_action() -> OperatorAction {
        OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
    }

    fn escalate_action() -> OperatorAction {
        OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::LowConfidence,
            ModelId::parse("claude-opus-4-7").unwrap(),
        ))
    }

    #[test]
    fn tool_returns_some_only_for_tool_call() {
        assert_eq!(tool_call_action().tool(), Some(KernelTool::Inspect));
        assert_eq!(stop_action().tool(), None);
        assert_eq!(escalate_action().tool(), None);
    }

    #[test]
    fn consumes_tool_slot_is_true_only_for_tool_call() {
        assert!(tool_call_action().consumes_tool_slot());
        assert!(!stop_action().consumes_tool_slot());
        assert!(!escalate_action().consumes_tool_slot());
    }

    #[test]
    fn kind_reports_variant_label() {
        assert_eq!(tool_call_action().kind().as_str(), "tool_call");
        assert_eq!(stop_action().kind().as_str(), "stop");
        assert_eq!(escalate_action().kind().as_str(), "escalate");
    }
}
