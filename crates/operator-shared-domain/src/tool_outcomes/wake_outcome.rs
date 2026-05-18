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
}

impl WakeOutcome {
    pub fn new(summary: NonEmptyString, surfaced_refs: Vec<MemoryRef>) -> Self {
        Self {
            summary,
            surfaced_refs,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn surfaced_refs(&self) -> &[MemoryRef] {
        &self.surfaced_refs
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
    }
}
