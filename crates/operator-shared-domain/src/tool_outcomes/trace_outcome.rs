//! Typed projection of `kernel_trace` structured content. First-pass
//! shape: a summary plus the ordered ref chain that connects the
//! requested `from` to `to`. The full proof-path metadata (relation
//! type, confidence, evidence pointers) drops on the floor for now.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceOutcome {
    summary: NonEmptyString,
    path_refs: Vec<MemoryRef>,
}

impl TraceOutcome {
    pub fn new(summary: NonEmptyString, path_refs: Vec<MemoryRef>) -> Self {
        Self { summary, path_refs }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn path_refs(&self) -> &[MemoryRef] {
        &self.path_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_summary_and_path() {
        let outcome = TraceOutcome::new(
            NonEmptyString::parse("trace summary", "trace_outcome.summary").unwrap(),
            vec![
                MemoryRef::parse("claim:a").unwrap(),
                MemoryRef::parse("claim:b").unwrap(),
            ],
        );
        assert_eq!(outcome.summary().as_str(), "trace summary");
        assert_eq!(outcome.path_refs().len(), 2);
    }
}
