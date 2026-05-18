//! Typed projection of `kernel_inspect` structured content. First-pass
//! shape: the target ref + its kind, plus the typed links going in and
//! out of the node. Inline evidence and raw audit fields drop on the
//! floor; a later pass adds them when an evaluation rule needs them.

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectOutcome {
    summary: NonEmptyString,
    target: MemoryRef,
    kind: NonEmptyString,
    incoming_refs: Vec<MemoryRef>,
    outgoing_refs: Vec<MemoryRef>,
}

impl InspectOutcome {
    pub fn new(
        summary: NonEmptyString,
        target: MemoryRef,
        kind: NonEmptyString,
        incoming_refs: Vec<MemoryRef>,
        outgoing_refs: Vec<MemoryRef>,
    ) -> Self {
        Self {
            summary,
            target,
            kind,
            incoming_refs,
            outgoing_refs,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn target(&self) -> &MemoryRef {
        &self.target
    }

    pub fn kind(&self) -> &NonEmptyString {
        &self.kind
    }

    pub fn incoming_refs(&self) -> &[MemoryRef] {
        &self.incoming_refs
    }

    pub fn outgoing_refs(&self) -> &[MemoryRef] {
        &self.outgoing_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_every_field() {
        let outcome = InspectOutcome::new(
            NonEmptyString::parse("inspect summary", "inspect_outcome.summary").unwrap(),
            MemoryRef::parse("claim:rachel-austin").unwrap(),
            NonEmptyString::parse("claim", "inspect_outcome.kind").unwrap(),
            vec![MemoryRef::parse("claim:incoming").unwrap()],
            vec![MemoryRef::parse("claim:outgoing").unwrap()],
        );
        assert_eq!(outcome.target().as_str(), "claim:rachel-austin");
        assert_eq!(outcome.kind().as_str(), "claim");
        assert_eq!(outcome.incoming_refs().len(), 1);
        assert_eq!(outcome.outgoing_refs().len(), 1);
    }
}
