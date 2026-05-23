//! One parsed teacher decision plus provider metadata needed for auditing.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherDecision {
    action: OperatorAction,
    finish_reason: FinishReason,
    subject_hash: SubjectHash,
}

impl TeacherDecision {
    pub fn new(
        action: OperatorAction,
        finish_reason: FinishReason,
        subject_hash: SubjectHash,
    ) -> Self {
        Self {
            action,
            finish_reason,
            subject_hash,
        }
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }

    pub fn into_action(self) -> OperatorAction {
        self.action
    }

    pub fn finish_reason(&self) -> FinishReason {
        self.finish_reason
    }

    pub fn subject_hash(&self) -> &SubjectHash {
        &self.subject_hash
    }
}
