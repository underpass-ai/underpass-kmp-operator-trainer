use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestSourceKind {
    Human,
    Agent,
    Projection,
    Derived,
}

impl IngestSourceKind {
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "human" => Ok(Self::Human),
            "agent" => Ok(Self::Agent),
            "projection" => Ok(Self::Projection),
            "derived" => Ok(Self::Derived),
            other => Err(DomainError::UnsupportedValue {
                context: "ingest.provenance.source_kind",
                value: other.to_string(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::Projection => "projection",
            Self::Derived => "derived",
        }
    }
}
