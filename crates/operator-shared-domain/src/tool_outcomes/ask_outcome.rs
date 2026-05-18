//! Typed projection of `kernel_ask` structured content. `answer = None`
//! mirrors the MCP UNKNOWN response (the kernel could not produce a
//! deterministic answer); `evidence_refs` are the refs the kernel
//! traversed to back the answer (empty when UNKNOWN).

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskOutcome {
    summary: NonEmptyString,
    answer: Option<NonEmptyString>,
    evidence_refs: Vec<MemoryRef>,
}

impl AskOutcome {
    pub fn new(
        summary: NonEmptyString,
        answer: Option<NonEmptyString>,
        evidence_refs: Vec<MemoryRef>,
    ) -> Self {
        Self {
            summary,
            answer,
            evidence_refs,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn answer(&self) -> Option<&NonEmptyString> {
        self.answer.as_ref()
    }

    pub fn evidence_refs(&self) -> &[MemoryRef] {
        &self.evidence_refs
    }

    pub fn is_unknown(&self) -> bool {
        self.answer.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(text: &str) -> NonEmptyString {
        NonEmptyString::parse(text, "ask_outcome.summary").unwrap()
    }

    #[test]
    fn answered_outcome_exposes_answer_and_evidence() {
        let evidence = MemoryRef::parse("evidence:1").unwrap();
        let outcome = AskOutcome::new(
            summary("ask summary"),
            Some(NonEmptyString::parse("the answer", "ask_outcome.answer").unwrap()),
            vec![evidence.clone()],
        );
        assert!(!outcome.is_unknown());
        assert_eq!(outcome.answer().unwrap().as_str(), "the answer");
        assert_eq!(outcome.evidence_refs(), &[evidence]);
    }

    #[test]
    fn unknown_outcome_has_no_answer() {
        let outcome = AskOutcome::new(summary("no answer found"), None, vec![]);
        assert!(outcome.is_unknown());
        assert!(outcome.answer().is_none());
        assert!(outcome.evidence_refs().is_empty());
    }
}
