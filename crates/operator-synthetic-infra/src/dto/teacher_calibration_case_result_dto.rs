use serde::{Deserialize, Serialize};

use crate::dto::teacher_calibration_prediction_outcome_dto::TeacherCalibrationPredictionOutcomeDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherCalibrationCaseResultDto {
    pub case_id: String,
    pub capability: String,
    pub category: String,
    pub outcome: TeacherCalibrationPredictionOutcomeDto,
    pub shape_failed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_action_rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
}
