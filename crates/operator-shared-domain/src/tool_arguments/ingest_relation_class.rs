use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestRelationClass {
    Structural,
    Causal,
    Motivational,
    Procedural,
    Evidential,
    Constraint,
}

impl IngestRelationClass {
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "structural" => Ok(Self::Structural),
            "causal" => Ok(Self::Causal),
            "motivational" => Ok(Self::Motivational),
            "procedural" => Ok(Self::Procedural),
            "evidential" => Ok(Self::Evidential),
            "constraint" => Ok(Self::Constraint),
            other => Err(DomainError::UnsupportedValue {
                context: "ingest.memory.relations[].class",
                value: other.to_string(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Structural => "structural",
            Self::Causal => "causal",
            Self::Motivational => "motivational",
            Self::Procedural => "procedural",
            Self::Evidential => "evidential",
            Self::Constraint => "constraint",
        }
    }

    pub fn is_structural(self) -> bool {
        self == Self::Structural
    }
}
