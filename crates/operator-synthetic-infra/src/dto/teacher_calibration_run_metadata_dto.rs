use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeacherCalibrationRunMetadataDto {
    pub predictor: String,
    pub dataset_path: String,
    pub dataset_sha256: String,
    pub prompt_path: String,
    pub prompt_sha256: String,
    pub api_base: String,
    pub model: String,
    pub temperature: f32,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
}
