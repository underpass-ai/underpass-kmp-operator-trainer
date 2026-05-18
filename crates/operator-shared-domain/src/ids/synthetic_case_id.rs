use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyntheticCaseId {
    inner: NonEmptyString,
}

impl SyntheticCaseId {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "synthetic_case_id")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for SyntheticCaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}
