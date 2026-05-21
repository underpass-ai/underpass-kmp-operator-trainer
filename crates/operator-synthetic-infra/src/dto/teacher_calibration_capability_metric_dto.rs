use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeacherCalibrationCapabilityMetricDto {
    pub total: usize,
    pub matches: usize,
    pub accuracy: Option<f64>,
}
