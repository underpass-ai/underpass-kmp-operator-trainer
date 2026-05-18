//! Minimal first-pass representation of write-memory arguments. The
//! payload is modelled as `summary`, `body`, and a list of related memory
//! refs. Full relation typing (relation types, qualities, weights) is
//! intentionally out of scope until the synthetic context lands.

use crate::error::domain_result::DomainResult;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteMemoryArguments {
    summary: NonEmptyString,
    body: NonEmptyString,
    related: Vec<MemoryRef>,
}

impl WriteMemoryArguments {
    pub fn new(
        summary: impl Into<String>,
        body: impl Into<String>,
        related: Vec<MemoryRef>,
    ) -> DomainResult<Self> {
        Ok(Self {
            summary: NonEmptyString::parse(summary, "write_memory_arguments.summary")?,
            body: NonEmptyString::parse(body, "write_memory_arguments.body")?,
            related,
        })
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn body(&self) -> &NonEmptyString {
        &self.body
    }

    pub fn related(&self) -> &[MemoryRef] {
        &self.related
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::domain_error::DomainError;

    #[test]
    fn refuses_empty_summary() {
        let err = WriteMemoryArguments::new("", "body", vec![]).unwrap_err();
        assert!(
            matches!(err, DomainError::EmptyValue { context } if context == "write_memory_arguments.summary")
        );
    }

    #[test]
    fn refuses_empty_body() {
        let err = WriteMemoryArguments::new("summary", "", vec![]).unwrap_err();
        assert!(
            matches!(err, DomainError::EmptyValue { context } if context == "write_memory_arguments.body")
        );
    }

    #[test]
    fn carries_summary_body_and_related() {
        let related = vec![MemoryRef::parse("node:1").unwrap()];
        let args = WriteMemoryArguments::new("s", "b", related.clone()).unwrap();
        assert_eq!(args.summary().as_str(), "s");
        assert_eq!(args.body().as_str(), "b");
        assert_eq!(args.related(), related.as_slice());
    }
}
