//! Realistic corpus scenario: a generation target plus the visible KMP state
//! the teacher may use to choose one Operator action.

use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::case::synthetic_acceptance_criteria::SyntheticAcceptanceCriteria;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::ports::scenario_id::ScenarioId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    id: ScenarioId,
    target: SyntheticGenerationTarget,
    subject: CalibrationSubject,
    acceptance_criteria: SyntheticAcceptanceCriteria,
    subject_hash: SubjectHash,
}

impl Scenario {
    pub fn new(
        id: ScenarioId,
        target: SyntheticGenerationTarget,
        subject: CalibrationSubject,
        acceptance_criteria: SyntheticAcceptanceCriteria,
        subject_hash: SubjectHash,
    ) -> Self {
        Self {
            id,
            target,
            subject,
            acceptance_criteria,
            subject_hash,
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

    pub fn acceptance_criteria(&self) -> &SyntheticAcceptanceCriteria {
        &self.acceptance_criteria
    }

    pub fn subject_hash(&self) -> &SubjectHash {
        &self.subject_hash
    }
}
