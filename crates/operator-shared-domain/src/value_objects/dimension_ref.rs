use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionRef {
    inner: NonEmptyString,
}

impl DimensionRef {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "dimension_ref")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for DimensionRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::domain_error::DomainError;

    #[test]
    fn refuses_empty_dimension_ref() {
        assert_eq!(
            DimensionRef::parse(""),
            Err(DomainError::EmptyValue {
                context: "dimension_ref"
            })
        );
    }
}
