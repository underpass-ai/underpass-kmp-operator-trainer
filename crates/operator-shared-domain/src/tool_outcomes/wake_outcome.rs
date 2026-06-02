//! Typed projection of the structured content returned by `kernel_wake`.
//! First-pass shape: the summary line plus every memory ref surfaced in
//! the packet, in stable order. Other fields (objective, causal spine,
//! next actions, proof) drop on the floor and can be added by a later
//! pass when a use case needs them.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeOutcome {
    summary: NonEmptyString,
    surfaced_refs: Vec<MemoryRef>,
    /// Entries the kernel knows about but withheld from this packet's evidence
    /// (`proof.frontier_size`). Non-zero only when wake ran with an opt-in entry
    /// window; it is the operator's signal that the about's tail is reachable by
    /// near-expansion. 0 = the packet surfaced the complete about.
    frontier_size: usize,
}

impl WakeOutcome {
    pub fn new(summary: NonEmptyString, surfaced_refs: Vec<MemoryRef>) -> Self {
        Self {
            summary,
            surfaced_refs,
            frontier_size: 0,
        }
    }

    /// Record the unsurfaced tail size reported by the kernel
    /// (`proof.frontier_size`) so the operator can decide to near-expand.
    #[must_use]
    pub fn with_frontier_size(mut self, frontier_size: usize) -> Self {
        self.frontier_size = frontier_size;
        self
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn surfaced_refs(&self) -> &[MemoryRef] {
        &self.surfaced_refs
    }

    pub fn frontier_size(&self) -> usize {
        self.frontier_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_summary_and_surfaced_refs() {
        let summary = NonEmptyString::parse("wake summary", "wake_outcome.summary").unwrap();
        let target = MemoryRef::parse("node:1").unwrap();
        let outcome = WakeOutcome::new(summary, vec![target.clone()]);
        assert_eq!(outcome.summary().as_str(), "wake summary");
        assert_eq!(outcome.surfaced_refs(), &[target]);
        assert_eq!(outcome.frontier_size(), 0, "unbounded wake by default");
    }

    #[test]
    fn records_the_frontier_size_tail() {
        let summary = NonEmptyString::parse("wake summary", "wake_outcome.summary").unwrap();
        let outcome = WakeOutcome::new(summary, vec![]).with_frontier_size(9);
        assert_eq!(outcome.frontier_size(), 9);
    }
}
