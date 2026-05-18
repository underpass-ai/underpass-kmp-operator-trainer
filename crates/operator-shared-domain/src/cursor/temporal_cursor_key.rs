use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemporalCursorKey {
    Created,
    Updated,
    Accessed,
}

impl TemporalCursorKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Accessed => "accessed",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "created" => Ok(Self::Created),
            "updated" => Ok(Self::Updated),
            "accessed" => Ok(Self::Accessed),
            other => Err(DomainError::UnsupportedValue {
                context: "temporal_cursor_key",
                value: other.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for TemporalCursorKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_each_variant() {
        for variant in [
            TemporalCursorKey::Created,
            TemporalCursorKey::Updated,
            TemporalCursorKey::Accessed,
        ] {
            assert_eq!(TemporalCursorKey::parse(variant.as_str()).unwrap(), variant);
        }
    }

    #[test]
    fn refuses_unknown_key() {
        assert!(matches!(
            TemporalCursorKey::parse("seen"),
            Err(DomainError::UnsupportedValue {
                context: "temporal_cursor_key",
                ..
            })
        ));
    }
}
