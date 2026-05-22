use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealisticCorpusRunMetadataDto {
    pub predictor: String,
    pub run_id: String,
    pub scenarios_path: String,
    pub scenarios_sha256: String,
    pub prompt_path: String,
    pub prompt_sha256: String,
    pub api_base: String,
    pub model: String,
    pub temperature: f32,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
}
