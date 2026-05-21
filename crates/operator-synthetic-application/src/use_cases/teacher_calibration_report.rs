//! Report produced by teacher calibration evaluation.

use std::collections::BTreeMap;

use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_synthetic_domain::calibration::calibration_capability::CalibrationCapability;
use operator_synthetic_domain::calibration::calibration_case_category::CalibrationCaseCategory;

use crate::use_cases::teacher_calibration_capability_metric::TeacherCalibrationCapabilityMetric;
use crate::use_cases::teacher_calibration_case_result::TeacherCalibrationCaseResult;
use crate::use_cases::teacher_calibration_category_metric::TeacherCalibrationCategoryMetric;
use crate::use_cases::teacher_calibration_ratio::accuracy_ratio;

const OVERALL_THRESHOLD: f64 = 0.80;
const CAPABILITY_FLOOR: f64 = 0.60;

#[derive(Debug, Clone, PartialEq)]
pub struct TeacherCalibrationReport {
    case_results: Vec<TeacherCalibrationCaseResult>,
    per_capability: BTreeMap<CalibrationCapability, TeacherCalibrationCapabilityMetric>,
    per_category: BTreeMap<CalibrationCaseCategory, TeacherCalibrationCategoryMetric>,
    gate_failure_reason: Option<NonEmptyString>,
}

impl TeacherCalibrationReport {
    pub fn from_case_results(case_results: Vec<TeacherCalibrationCaseResult>) -> Self {
        let per_capability = build_metrics(&case_results);
        let per_category = build_category_metrics(&case_results);
        let gate_failure_reason = gate_failure_reason(&case_results, &per_capability);
        Self {
            case_results,
            per_capability,
            per_category,
            gate_failure_reason,
        }
    }

    pub fn case_results(&self) -> &[TeacherCalibrationCaseResult] {
        &self.case_results
    }

    pub fn per_capability(
        &self,
    ) -> &BTreeMap<CalibrationCapability, TeacherCalibrationCapabilityMetric> {
        &self.per_capability
    }

    pub fn per_category(
        &self,
    ) -> &BTreeMap<CalibrationCaseCategory, TeacherCalibrationCategoryMetric> {
        &self.per_category
    }

    pub fn total_cases(&self) -> ExampleCount {
        ExampleCount::new(self.case_results.len())
    }

    pub fn match_count(&self) -> ExampleCount {
        ExampleCount::new(self.case_results.iter().filter(|row| row.matched()).count())
    }

    pub fn tool_match_count(&self) -> ExampleCount {
        ExampleCount::new(
            self.case_results
                .iter()
                .filter(|row| row.tool_matched())
                .count(),
        )
    }

    pub fn contract_valid_count(&self) -> ExampleCount {
        ExampleCount::new(
            self.case_results
                .iter()
                .filter(|row| row.contract_valid())
                .count(),
        )
    }

    pub fn shape_failed_count(&self) -> ExampleCount {
        ExampleCount::new(
            self.case_results
                .iter()
                .filter(|row| row.shape_failed())
                .count(),
        )
    }

    pub fn overall_accuracy(&self) -> f64 {
        if self.case_results.is_empty() {
            return 0.0;
        }
        accuracy_ratio(self.match_count().as_usize(), self.case_results.len())
            .expect("non-empty calibration report has accuracy ratio")
    }

    pub fn gate_passed(&self) -> bool {
        self.gate_failure_reason.is_none()
    }

    pub fn gate_failure_reason(&self) -> Option<&NonEmptyString> {
        self.gate_failure_reason.as_ref()
    }
}

fn build_category_metrics(
    case_results: &[TeacherCalibrationCaseResult],
) -> BTreeMap<CalibrationCaseCategory, TeacherCalibrationCategoryMetric> {
    let mut metrics = BTreeMap::new();
    for category in CalibrationCaseCategory::ALL {
        let total = case_results
            .iter()
            .filter(|row| row.category() == category)
            .count();
        let matches = case_results
            .iter()
            .filter(|row| row.category() == category && row.matched())
            .count();
        metrics.insert(
            category,
            TeacherCalibrationCategoryMetric::new(
                category,
                ExampleCount::new(total),
                ExampleCount::new(matches),
            ),
        );
    }
    metrics
}

fn build_metrics(
    case_results: &[TeacherCalibrationCaseResult],
) -> BTreeMap<CalibrationCapability, TeacherCalibrationCapabilityMetric> {
    let mut metrics = BTreeMap::new();
    for capability in CalibrationCapability::ALL {
        let total = case_results
            .iter()
            .filter(|row| row.capability() == capability)
            .count();
        let matches = case_results
            .iter()
            .filter(|row| row.capability() == capability && row.matched())
            .count();
        metrics.insert(
            capability,
            TeacherCalibrationCapabilityMetric::new(
                capability,
                ExampleCount::new(total),
                ExampleCount::new(matches),
            ),
        );
    }
    metrics
}

fn gate_failure_reason(
    case_results: &[TeacherCalibrationCaseResult],
    per_capability: &BTreeMap<CalibrationCapability, TeacherCalibrationCapabilityMetric>,
) -> Option<NonEmptyString> {
    if case_results.is_empty() {
        return Some(
            NonEmptyString::parse("no calibration cases were evaluated", "gate_failure_reason")
                .unwrap(),
        );
    }
    let overall = accuracy_ratio(
        case_results.iter().filter(|row| row.matched()).count(),
        case_results.len(),
    )
    .expect("non-empty calibration report has accuracy ratio");
    if overall < OVERALL_THRESHOLD {
        return Some(
            NonEmptyString::parse(
                format!("overall_accuracy {overall:.2} < threshold {OVERALL_THRESHOLD:.2}"),
                "gate_failure_reason",
            )
            .unwrap(),
        );
    }
    for capability in CalibrationCapability::ALL {
        let metric = per_capability
            .get(&capability)
            .expect("metric for every capability");
        let Some(accuracy) = metric.accuracy() else {
            return Some(
                NonEmptyString::parse(
                    format!("{} has no calibration cases", capability.as_str()),
                    "gate_failure_reason",
                )
                .unwrap(),
            );
        };
        if accuracy < CAPABILITY_FLOOR {
            return Some(
                NonEmptyString::parse(
                    format!(
                        "{} per_capability_accuracy {accuracy:.2} < floor {CAPABILITY_FLOOR:.2}",
                        capability.as_str()
                    ),
                    "gate_failure_reason",
                )
                .unwrap(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_synthetic_domain::calibration::calibration_case_id::CalibrationCaseId;

    use crate::use_cases::teacher_calibration_prediction_outcome::TeacherCalibrationPredictionOutcome;

    #[test]
    fn empty_report_fails_gate() {
        let report = TeacherCalibrationReport::from_case_results(vec![]);
        assert!(!report.gate_passed());
        assert_eq!(report.total_cases().as_usize(), 0);
    }

    #[test]
    fn report_counts_case_outcomes() {
        let report = TeacherCalibrationReport::from_case_results(vec![result(
            CalibrationCapability::KernelInspect,
            true,
        )]);
        assert_eq!(report.match_count().as_usize(), 1);
        assert_eq!(report.tool_match_count().as_usize(), 1);
        assert_eq!(report.contract_valid_count().as_usize(), 1);
        assert_eq!(report.shape_failed_count().as_usize(), 0);
        assert!((report.overall_accuracy() - 1.0).abs() < f64::EPSILON);
        assert_eq!(
            report
                .per_category()
                .get(&CalibrationCaseCategory::Happy)
                .unwrap()
                .total()
                .as_usize(),
            1
        );
        assert!(!report.gate_passed());
    }

    fn result(capability: CalibrationCapability, matched: bool) -> TeacherCalibrationCaseResult {
        TeacherCalibrationCaseResult::prediction(
            CalibrationCaseId::parse(format!("calib:{}", capability.as_str())).unwrap(),
            capability,
            CalibrationCaseCategory::Happy,
            TeacherCalibrationPredictionOutcome::new(matched, matched, true),
            None,
            None,
        )
    }
}
