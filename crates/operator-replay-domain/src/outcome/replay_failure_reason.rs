//! Domain-side cause for a failed `ToolCall` execution. Adapters
//! translate `operator-replay-application::KmpClientError` variants
//! into one of these reasons so the report stays adapter-agnostic.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplayFailureReason {
    Transport,
    Protocol,
    InvalidArguments,
    MalformedResponse,
}

impl ReplayFailureReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::Protocol => "protocol",
            Self::InvalidArguments => "invalid_arguments",
            Self::MalformedResponse => "malformed_response",
        }
    }
}

impl std::fmt::Display for ReplayFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
