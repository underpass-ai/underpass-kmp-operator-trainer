use crate::contract::correctness::correctness_mode::CorrectnessMode;
use crate::contract::correctness::field_outcome::FieldOutcome;
use crate::contract::correctness::field_path::FieldPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldResult {
    field_path: FieldPath,
    mode: CorrectnessMode,
    outcome: FieldOutcome,
}

impl FieldResult {
    pub fn new(field_path: FieldPath, mode: CorrectnessMode, outcome: FieldOutcome) -> Self {
        Self {
            field_path,
            mode,
            outcome,
        }
    }

    pub fn field_path(&self) -> &FieldPath {
        &self.field_path
    }

    pub fn mode(&self) -> CorrectnessMode {
        self.mode
    }

    pub fn outcome(&self) -> &FieldOutcome {
        &self.outcome
    }

    pub fn is_correct(&self) -> bool {
        self.outcome.is_pass()
    }
}
