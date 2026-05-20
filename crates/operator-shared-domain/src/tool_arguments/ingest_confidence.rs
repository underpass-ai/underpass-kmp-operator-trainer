use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

// kernel-source:
// - api/proto/underpass/rehydration/kernel/v1beta1/memory.proto:422-428 defines MemoryConfidence.
// - crates/rehydration-mcp/src/grpc/requests/common.rs:294-309 maps MCP strings to proto enum.
// - crates/rehydration-mcp/src/grpc/requests/ingest.rs:173-181 makes non-structural confidence required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestConfidence {
    High,
    Medium,
    Low,
    Unknown,
}

impl IngestConfidence {
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            "unknown" => Ok(Self::Unknown),
            other => Err(DomainError::UnsupportedValue {
                context: "ingest.memory.relations[].confidence",
                value: other.to_string(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Unknown => "unknown",
        }
    }
}
