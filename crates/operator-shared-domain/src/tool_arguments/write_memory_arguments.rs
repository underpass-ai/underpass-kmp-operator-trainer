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
