//! Audit row for one scenario dropped from realistic corpus output.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::ports::scenario_id::ScenarioId;
use crate::use_cases::drop_reason::DropReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropEntry {
    scenario_id: ScenarioId,
    target: SyntheticGenerationTarget,
    reason: DropReason,
    predicted_action: Option<OperatorAction>,
    subject_hash: SubjectHash,
    teacher_finish_reason: Option<FinishReason>,
}

impl DropEntry {
    pub fn new(
        scenario_id: ScenarioId,
        target: SyntheticGenerationTarget,
        reason: DropReason,
        predicted_action: Option<OperatorAction>,
        subject_hash: SubjectHash,
        teacher_finish_reason: Option<FinishReason>,
    ) -> Self {
        debug_assert!(
            predicted_action.is_some()
                || matches!(
                    reason,
                    DropReason::TeacherError { .. } | DropReason::ParseFailure { .. }
                ),
            "drops with parsed teacher actions must persist predicted_action"
        );
        Self {
            scenario_id,
            target,
            reason,
            predicted_action,
            subject_hash,
            teacher_finish_reason,
        }
    }

    pub fn scenario_id(&self) -> &ScenarioId {
        &self.scenario_id
    }

    pub fn target(&self) -> SyntheticGenerationTarget {
        self.target
    }

    pub fn reason(&self) -> &DropReason {
        &self.reason
    }

    pub fn predicted_action(&self) -> Option<&OperatorAction> {
        self.predicted_action.as_ref()
    }

    pub fn subject_hash(&self) -> &SubjectHash {
        &self.subject_hash
    }

    pub fn teacher_finish_reason(&self) -> Option<FinishReason> {
        self.teacher_finish_reason
    }
}
