use serde::{Deserialize, Serialize};

use crate::dto::calibration_subject_dto::CalibrationSubjectDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioDto {
    pub scenario_id: String,
    pub target: String,
    pub subject: CalibrationSubjectDto,
}
