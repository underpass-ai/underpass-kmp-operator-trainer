//! Per-capability teacher calibration metric.

use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_synthetic_domain::calibration::calibration_capability::CalibrationCapability;

use crate::use_cases::teacher_calibration_ratio::accuracy_ratio;

#[derive(Debug, Clone, PartialEq)]
pub struct TeacherCalibrationCapabilityMetric {
    capability: CalibrationCapability,
    total: ExampleCount,
    matches: ExampleCount,
}

impl TeacherCalibrationCapabilityMetric {
    pub fn new(
        capability: CalibrationCapability,
        total: ExampleCount,
        matches: ExampleCount,
    ) -> Self {
        Self {
            capability,
            total,
            matches,
        }
    }

    pub fn capability(&self) -> CalibrationCapability {
        self.capability
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
