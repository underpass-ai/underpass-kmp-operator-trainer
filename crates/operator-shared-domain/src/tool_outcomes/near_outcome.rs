//! Typed projection of `kernel_near` structured content. First-pass
//! shape: the summary line plus the refs that appeared near the
//! requested temporal anchor, in the order returned by the kernel.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearOutcome {
    summary: NonEmptyString,
    entry_refs: Vec<MemoryRef>,
}

impl NearOutcome {
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
        let outcome = NearOutcome::new(
            NonEmptyString::parse("near summary", "near_outcome.summary").unwrap(),
            vec![MemoryRef::parse("entry:1").unwrap()],
        );
        assert_eq!(outcome.summary().as_str(), "near summary");
        assert_eq!(outcome.entry_refs().len(), 1);
    }
}
