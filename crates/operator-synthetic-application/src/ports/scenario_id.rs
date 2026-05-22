//! Identifier for one externally authored realistic corpus scenario.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScenarioId {
    inner: NonEmptyString,
}

impl ScenarioId {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "scenario_id")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for ScenarioId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_id() {
        assert!(ScenarioId::parse("").is_err());
    }
}
