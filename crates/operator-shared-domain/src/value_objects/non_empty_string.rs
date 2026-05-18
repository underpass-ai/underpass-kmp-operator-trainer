//! Non-empty UTF-8 string backing for every named string value object in the
//! shared domain. Other value objects compose `NonEmptyString` rather than
//! re-implementing the empty/whitespace check.

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyString {
    raw: String,
}

impl NonEmptyString {
    pub fn parse(value: impl Into<String>, context: &'static str) -> DomainResult<Self> {
        let raw = value.into();
        if raw.trim().is_empty() {
            return Err(DomainError::EmptyValue { context });
        }
        Ok(Self { raw })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl std::fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_value() {
        assert_eq!(
            NonEmptyString::parse("", "test"),
            Err(DomainError::EmptyValue { context: "test" })
        );
    }

    #[test]
    fn refuses_whitespace_only_value() {
        assert_eq!(
            NonEmptyString::parse("   ", "test"),
            Err(DomainError::EmptyValue { context: "test" })
        );
    }

    #[test]
    fn accepts_non_empty_value() {
        let value = NonEmptyString::parse("foo", "test").expect("foo is valid");
        assert_eq!(value.as_str(), "foo");
        assert_eq!(format!("{value}"), "foo");
    }
}
