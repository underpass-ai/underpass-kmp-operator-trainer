#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalReason {
    Stop,
    Escalate,
}

impl TerminalReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Escalate => "escalate",
        }
    }
}

impl std::fmt::Display for TerminalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
