use crate::use_cases::trajectory_validation_failure::TrajectoryValidationFailure;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    total: usize,
    successful: usize,
    failures: Vec<TrajectoryValidationFailure>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&mut self) {
        self.total += 1;
        self.successful += 1;
    }

    pub fn record_failure(&mut self, failure: TrajectoryValidationFailure) {
        self.total += 1;
        self.failures.push(failure);
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn successful(&self) -> usize {
        self.successful
    }

    pub fn failures(&self) -> &[TrajectoryValidationFailure] {
        &self.failures
    }

    pub fn all_passed(&self) -> bool {
        self.failures.is_empty()
    }
}
