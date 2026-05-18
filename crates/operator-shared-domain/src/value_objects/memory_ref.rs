use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryRef {
    inner: NonEmptyString,
}

impl MemoryRef {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "memory_ref")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for MemoryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::domain_error::DomainError;

    #[test]
    fn refuses_empty_memory_ref() {
        assert_eq!(
            MemoryRef::parse(""),
            Err(DomainError::EmptyValue {
                context: "memory_ref"
            })
        );
    }

    #[test]
    fn accepts_named_memory_ref() {
        let value = MemoryRef::parse("node:alpha").expect("named ref is valid");
        assert_eq!(value.as_str(), "node:alpha");
    }
}
