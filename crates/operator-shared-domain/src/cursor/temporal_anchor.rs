use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

/// Opaque anchor that identifies a position along a temporal axis. We do
/// not commit to a wall-clock representation in the domain; the anchor is
/// produced by the KMP server and the operator only carries it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporalAnchor {
    inner: NonEmptyString,
}

impl TemporalAnchor {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "temporal_anchor")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for TemporalAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}
