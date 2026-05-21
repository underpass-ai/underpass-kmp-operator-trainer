use operator_shared_contract::visible_state_dto::VisibleStateDto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSubjectDto {
    pub about: String,
    pub mode: String,
    pub task_family: String,
    pub goal: String,
    pub allowed_tools: Vec<String>,
    pub visible_state: VisibleStateDto,
}
