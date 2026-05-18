use crate::action::escalate_reason::EscalateReason;
use crate::value_objects::model_id::ModelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscalateAction {
    reason: EscalateReason,
    target_model: ModelId,
}

impl EscalateAction {
    pub fn new(reason: EscalateReason, target_model: ModelId) -> Self {
        Self {
            reason,
            target_model,
        }
    }

    pub fn reason(&self) -> EscalateReason {
        self.reason
    }

    pub fn target_model(&self) -> &ModelId {
        &self.target_model
    }
}
