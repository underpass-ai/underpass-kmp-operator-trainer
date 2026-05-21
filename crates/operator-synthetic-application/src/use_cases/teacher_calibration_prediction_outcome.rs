//! Outcome of one parsed teacher prediction before report aggregation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeacherCalibrationPredictionOutcome {
    matched: bool,
    tool_matched: bool,
    contract_valid: bool,
}

impl TeacherCalibrationPredictionOutcome {
    pub fn new(matched: bool, tool_matched: bool, contract_valid: bool) -> Self {
        Self {
            matched,
            tool_matched,
            contract_valid,
        }
    }

    pub fn matched(self) -> bool {
        self.matched
    }

    pub fn tool_matched(self) -> bool {
        self.tool_matched
    }

    pub fn contract_valid(self) -> bool {
        self.contract_valid
    }
}
