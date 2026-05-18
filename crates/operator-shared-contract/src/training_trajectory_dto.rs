use serde::{Deserialize, Serialize};

use crate::operator_action_dto::OperatorActionDto;
use crate::visible_state_dto::VisibleStateDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingTrajectoryDto {
    pub id: String,
    pub step_id: String,
    pub about: String,
    pub mode: String,
    pub task_family: String,
    pub allowed_tools: Vec<String>,
    pub visible_state: VisibleStateDto,
    pub target_action: OperatorActionDto,
}
