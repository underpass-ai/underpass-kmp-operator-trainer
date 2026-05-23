use serde::{Deserialize, Serialize};

use crate::dto::calibration_subject_dto::CalibrationSubjectDto;
use crate::dto::synthetic_acceptance_criteria_dto::SyntheticAcceptanceCriteriaDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioDto {
    pub scenario_id: String,
    pub target: String,
    pub subject: CalibrationSubjectDto,
    #[serde(default)]
    pub acceptance_criteria: Option<SyntheticAcceptanceCriteriaDto>,
}
