use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherCalibrationPredictionOutcomeDto {
    pub matched: bool,
    pub tool_matched: bool,
    pub contract_valid: bool,
}
