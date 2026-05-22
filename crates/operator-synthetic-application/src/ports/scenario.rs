//! Realistic corpus scenario: a generation target plus the visible KMP state
//! the teacher may use to choose one Operator action.

use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::ports::scenario_id::ScenarioId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    id: ScenarioId,
    target: SyntheticGenerationTarget,
    subject: CalibrationSubject,
}

impl Scenario {
    pub fn new(
        id: ScenarioId,
        target: SyntheticGenerationTarget,
        subject: CalibrationSubject,
    ) -> Self {
        Self {
            id,
            target,
            subject,
        }
    }

    pub fn id(&self) -> &ScenarioId {
        &self.id
    }

    pub fn target(&self) -> SyntheticGenerationTarget {
        self.target
    }

    pub fn subject(&self) -> &CalibrationSubject {
        &self.subject
    }
}
