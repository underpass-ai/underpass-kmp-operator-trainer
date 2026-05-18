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
