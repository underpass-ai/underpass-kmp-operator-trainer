use operator_shared_contract::operator_action_dto::OperatorActionDto;
use serde::{Deserialize, Serialize};

use crate::dto::calibration_subject_dto::CalibrationSubjectDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationCaseDto {
    pub case_id: String,
    pub domain_theme: String,
    pub category: String,
    pub subject: CalibrationSubjectDto,
    pub accepted_actions: Vec<OperatorActionDto>,
    pub expected_action_rationale: String,
}
