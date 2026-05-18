//! Typed projection of `kernel_write_memory` structured content.
//! First-pass shape captures the operationally-relevant fields:
//! whether the write was accepted, whether it was a dry run, and the
//! refs the kernel would generate (or did generate). Relation-quality
//! diagnostics and the ingest preview drop on the floor for now.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteMemoryOutcome {
    summary: NonEmptyString,
    accepted: bool,
    dry_run: bool,
    generated_refs: Vec<MemoryRef>,
}

impl WriteMemoryOutcome {
    pub fn new(
        summary: NonEmptyString,
        accepted: bool,
        dry_run: bool,
        generated_refs: Vec<MemoryRef>,
    ) -> Self {
        Self {
            summary,
            accepted,
            dry_run,
            generated_refs,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn accepted(&self) -> bool {
        self.accepted
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn generated_refs(&self) -> &[MemoryRef] {
        &self.generated_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_preview_is_not_accepted() {
        let outcome = WriteMemoryOutcome::new(
            NonEmptyString::parse("dry-run preview", "write_memory_outcome.summary").unwrap(),
            false,
            true,
            vec![MemoryRef::parse("incident:m:entry:1").unwrap()],
        );
        assert!(!outcome.accepted());
        assert!(outcome.dry_run());
        assert_eq!(outcome.generated_refs().len(), 1);
    }

    #[test]
    fn accepted_commit_has_generated_refs() {
        let outcome = WriteMemoryOutcome::new(
            NonEmptyString::parse("committed", "write_memory_outcome.summary").unwrap(),
            true,
            false,
            vec![MemoryRef::parse("incident:m:entry:1").unwrap()],
        );
        assert!(outcome.accepted());
        assert!(!outcome.dry_run());
    }
}
