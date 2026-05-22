//! Audit row for one scenario dropped from realistic corpus output.

use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::ports::scenario_id::ScenarioId;
use crate::use_cases::drop_reason::DropReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropEntry {
    scenario_id: ScenarioId,
    target: SyntheticGenerationTarget,
    reason: DropReason,
}

impl DropEntry {
    pub fn new(
        scenario_id: ScenarioId,
        target: SyntheticGenerationTarget,
        reason: DropReason,
    ) -> Self {
        Self {
            scenario_id,
            target,
            reason,
        }
    }

    pub fn scenario_id(&self) -> &ScenarioId {
        &self.scenario_id
    }

    pub fn target(&self) -> SyntheticGenerationTarget {
        self.target
    }

    pub fn reason(&self) -> &DropReason {
        &self.reason
    }
}
