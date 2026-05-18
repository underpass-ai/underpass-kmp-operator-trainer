//! Typed projection of `kernel_forward` structured content. First-pass
//! shape mirrors `NearOutcome`: a summary plus the entries the kernel
//! returned moving forward from the requested anchor.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardOutcome {
    summary: NonEmptyString,
    entry_refs: Vec<MemoryRef>,
}

impl ForwardOutcome {
    pub fn new(summary: NonEmptyString, entry_refs: Vec<MemoryRef>) -> Self {
        Self {
            summary,
            entry_refs,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn entry_refs(&self) -> &[MemoryRef] {
        &self.entry_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_summary_and_entries() {
        let outcome = ForwardOutcome::new(
            NonEmptyString::parse("forward summary", "forward_outcome.summary").unwrap(),
            vec![MemoryRef::parse("entry:1").unwrap()],
        );
        assert_eq!(outcome.summary().as_str(), "forward summary");
        assert_eq!(outcome.entry_refs().len(), 1);
    }
}
