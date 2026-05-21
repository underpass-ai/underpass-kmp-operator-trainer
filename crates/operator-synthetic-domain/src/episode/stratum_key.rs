//! Named stratum used by stratified episode splits.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StratumKey {
    inner: NonEmptyString,
}

impl StratumKey {
    pub fn parse(value: impl Into<String>) -> SyntheticDomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "stratum_key")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_key() {
        let key = StratumKey::parse("theme").unwrap();
        assert_eq!(key.as_str(), "theme");
        assert_eq!(key.clone(), key);
    }

    #[test]
    fn rejects_blank_key() {
        assert!(StratumKey::parse(" ").is_err());
    }
}
