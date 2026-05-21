use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dto::teacher_calibration_capability_metric_dto::TeacherCalibrationCapabilityMetricDto;
use crate::dto::teacher_calibration_case_result_dto::TeacherCalibrationCaseResultDto;
use crate::dto::teacher_calibration_category_metric_dto::TeacherCalibrationCategoryMetricDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeacherCalibrationReportDto {
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
    pub total_cases: usize,
    pub match_count: usize,
    pub tool_match_count: usize,
    pub contract_valid_count: usize,
    pub shape_failed_count: usize,
    pub overall_accuracy: f64,
    pub per_capability_accuracy: BTreeMap<String, Option<f64>>,
    pub per_capability_total: BTreeMap<String, usize>,
    pub per_capability: BTreeMap<String, TeacherCalibrationCapabilityMetricDto>,
    pub per_category_accuracy: BTreeMap<String, Option<f64>>,
    pub per_category_total: BTreeMap<String, usize>,
    pub per_category: BTreeMap<String, TeacherCalibrationCategoryMetricDto>,
    pub gate_passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_failure_reason: Option<String>,
    pub case_results: Vec<TeacherCalibrationCaseResultDto>,
}
