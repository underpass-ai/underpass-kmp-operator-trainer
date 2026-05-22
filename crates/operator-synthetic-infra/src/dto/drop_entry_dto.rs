use serde::{Deserialize, Serialize};

use operator_shared_contract::operator_action_dto::OperatorActionDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropEntryDto {
    pub scenario_id: String,
    pub target: String,
    pub reason: String,
    pub message: String,
    pub predicted_action: Option<OperatorActionDto>,
    pub subject_hash: String,
    pub teacher_finish_reason: Option<String>,
}
