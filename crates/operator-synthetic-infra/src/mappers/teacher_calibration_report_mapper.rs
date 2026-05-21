//! Mapper from application report to serializable report DTO.

use std::collections::BTreeMap;

use operator_synthetic_application::use_cases::teacher_calibration_report::TeacherCalibrationReport;

use crate::dto::teacher_calibration_capability_metric_dto::TeacherCalibrationCapabilityMetricDto;
use crate::dto::teacher_calibration_case_result_dto::TeacherCalibrationCaseResultDto;
use crate::dto::teacher_calibration_category_metric_dto::TeacherCalibrationCategoryMetricDto;
use crate::dto::teacher_calibration_prediction_outcome_dto::TeacherCalibrationPredictionOutcomeDto;
use crate::dto::teacher_calibration_report_dto::TeacherCalibrationReportDto;
use crate::dto::teacher_calibration_run_metadata_dto::TeacherCalibrationRunMetadataDto;

#[derive(Debug)]
pub struct TeacherCalibrationReportMapper;

impl TeacherCalibrationReportMapper {
    pub fn to_dto(
        report: &TeacherCalibrationReport,
        metadata: TeacherCalibrationRunMetadataDto,
    ) -> TeacherCalibrationReportDto {
        let mut per_capability_accuracy = BTreeMap::new();
        let mut per_capability_total = BTreeMap::new();
        let mut per_capability = BTreeMap::new();
        let mut per_category_accuracy = BTreeMap::new();
        let mut per_category_total = BTreeMap::new();
        let mut per_category = BTreeMap::new();
        for (capability, metric) in report.per_capability() {
            let key = capability.as_str().to_string();
            per_capability_accuracy.insert(key.clone(), metric.accuracy());
            per_capability_total.insert(key.clone(), metric.total().as_usize());
            per_capability.insert(
                key,
                TeacherCalibrationCapabilityMetricDto {
                    total: metric.total().as_usize(),
                    matches: metric.matches().as_usize(),
                    accuracy: metric.accuracy(),
                },
            );
        }
        for (category, metric) in report.per_category() {
            let key = category.as_str().to_string();
            per_category_accuracy.insert(key.clone(), metric.accuracy());
            per_category_total.insert(key.clone(), metric.total().as_usize());
            per_category.insert(
                key,
                TeacherCalibrationCategoryMetricDto {
                    total: metric.total().as_usize(),
                    matches: metric.matches().as_usize(),
                    accuracy: metric.accuracy(),
                },
            );
        }
        TeacherCalibrationReportDto {
            predictor: metadata.predictor,
            dataset_path: metadata.dataset_path,
            dataset_sha256: metadata.dataset_sha256,
            prompt_path: metadata.prompt_path,
            prompt_sha256: metadata.prompt_sha256,
            api_base: metadata.api_base,
            model: metadata.model,
            temperature: metadata.temperature,
            started_at_unix: metadata.started_at_unix,
            finished_at_unix: metadata.finished_at_unix,
            total_cases: report.total_cases().as_usize(),
            match_count: report.match_count().as_usize(),
            tool_match_count: report.tool_match_count().as_usize(),
            contract_valid_count: report.contract_valid_count().as_usize(),
            shape_failed_count: report.shape_failed_count().as_usize(),
            overall_accuracy: report.overall_accuracy(),
            per_capability_accuracy,
            per_capability_total,
            per_capability,
            per_category_accuracy,
            per_category_total,
            per_category,
            gate_passed: report.gate_passed(),
            gate_failure_reason: report
                .gate_failure_reason()
                .map(|reason| reason.as_str().to_string()),
            case_results: report
                .case_results()
                .iter()
                .map(|row| TeacherCalibrationCaseResultDto {
                    case_id: row.case_id().as_str().to_string(),
                    capability: row.capability().as_str().to_string(),
                    category: row.category().as_str().to_string(),
                    outcome: TeacherCalibrationPredictionOutcomeDto {
                        matched: row.matched(),
                        tool_matched: row.tool_matched(),
                        contract_valid: row.contract_valid(),
                    },
                    shape_failed: row.shape_failed(),
                    expected_action_rationale: row
                        .expected_action_rationale()
                        .map(|rationale| rationale.as_str().to_string()),
                    failure_message: row
                        .failure_message()
                        .map(|message| message.as_str().to_string()),
                })
                .collect(),
        }
    }
}
