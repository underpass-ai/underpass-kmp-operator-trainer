/// Discriminator-only view of `Cursor`, useful for routing and tests.
use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CursorKind {
    Ref,
    Around,
    Temporal,
    Trace,
}

impl CursorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ref => "ref",
            Self::Around => "around",
            Self::Temporal => "temporal",
            Self::Trace => "trace",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "ref" => Ok(Self::Ref),
            "around" => Ok(Self::Around),
            "temporal" => Ok(Self::Temporal),
            "trace" => Ok(Self::Trace),
            other => Err(DomainError::UnsupportedValue {
                context: "cursor_kind",
                value: other.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for CursorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
