use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealisticCorpusReportDto {
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
    pub total_scenarios: usize,
    pub accepted_count: usize,
    pub dropped_count: usize,
    pub drop_rate: f64,
    pub max_drop_rate_gate: f64,
    pub dropped_by_reason: BTreeMap<String, usize>,
    pub per_target_accepted: BTreeMap<String, usize>,
    pub per_target_total: BTreeMap<String, usize>,
    pub gate_passed: bool,
    pub gate_failure_reason: Option<String>,
}
