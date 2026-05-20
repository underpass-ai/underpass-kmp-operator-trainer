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
