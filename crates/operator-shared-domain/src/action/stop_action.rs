use crate::action::stop_reason::StopReason;
use crate::error::domain_result::DomainResult;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopAction {
    reason: StopReason,
    answer: Option<NonEmptyString>,
    evidence: Vec<MemoryRef>,
}

impl StopAction {
    pub fn new(
        reason: StopReason,
        answer: Option<String>,
        evidence: Vec<MemoryRef>,
    ) -> DomainResult<Self> {
        let answer = match answer {
            Some(text) => Some(NonEmptyString::parse(text, "stop_action.answer")?),
            None => None,
        };
        Ok(Self {
            reason,
            answer,
            evidence,
        })
    }

    pub fn reason(&self) -> StopReason {
        self.reason
    }

    pub fn answer(&self) -> Option<&NonEmptyString> {
        self.answer.as_ref()
    }

    pub fn evidence(&self) -> &[MemoryRef] {
        &self.evidence
    }
}
