//! Finish reason reported by a teacher model provider.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCall,
    Other,
}

impl FinishReason {
    pub fn parse(value: &str) -> Self {
        match value {
            "stop" => Self::Stop,
            "length" => Self::Length,
            "content_filter" => Self::ContentFilter,
            "tool_calls" | "tool_call" | "function_call" => Self::ToolCall,
            _ => Self::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ContentFilter => "content_filter",
            Self::ToolCall => "tool_call",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_provider_values() {
        assert_eq!(FinishReason::parse("stop"), FinishReason::Stop);
        assert_eq!(FinishReason::parse("length"), FinishReason::Length);
        assert_eq!(
            FinishReason::parse("content_filter"),
            FinishReason::ContentFilter
        );
        assert_eq!(FinishReason::parse("tool_calls"), FinishReason::ToolCall);
    }

    #[test]
    fn unknown_values_are_kept_as_other_bucket() {
        assert_eq!(FinishReason::parse("unknown"), FinishReason::Other);
    }
}
