//! Per-category teacher calibration metric.

use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_synthetic_domain::calibration::calibration_case_category::CalibrationCaseCategory;

use crate::use_cases::teacher_calibration_ratio::accuracy_ratio;

#[derive(Debug, Clone, PartialEq)]
pub struct TeacherCalibrationCategoryMetric {
    category: CalibrationCaseCategory,
    total: ExampleCount,
    matches: ExampleCount,
}

impl TeacherCalibrationCategoryMetric {
    pub fn new(
        category: CalibrationCaseCategory,
        total: ExampleCount,
        matches: ExampleCount,
    ) -> Self {
        Self {
            category,
            total,
            matches,
        }
    }

    pub fn category(&self) -> CalibrationCaseCategory {
        self.category
    }

    pub fn total(&self) -> ExampleCount {
        self.total
    }

    pub fn matches(&self) -> ExampleCount {
        self.matches
    }

    pub fn accuracy(&self) -> Option<f64> {
        accuracy_ratio(self.matches.as_usize(), self.total.as_usize())
    }
}
