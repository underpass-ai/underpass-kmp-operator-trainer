use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscalateReason {
    AmbiguousIntent,
    BeyondCapability,
    LowConfidence,
}

impl EscalateReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AmbiguousIntent => "ambiguous_intent",
            Self::BeyondCapability => "beyond_capability",
            Self::LowConfidence => "low_confidence",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "ambiguous_intent" => Ok(Self::AmbiguousIntent),
            "beyond_capability" => Ok(Self::BeyondCapability),
            "low_confidence" => Ok(Self::LowConfidence),
            other => Err(DomainError::UnsupportedValue {
                context: "escalate_reason",
                value: other.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for EscalateReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
