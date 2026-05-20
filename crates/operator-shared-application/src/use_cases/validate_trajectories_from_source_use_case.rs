//! `ValidateTrajectoriesFromSourceUseCase` reads every trajectory from a
//! `TrajectoryReader` adapter and runs the contract validator on each.
//! It returns a `ValidationReport` aggregating successes and failures.
//!
//! This use case is the canonical demonstration of how the driving side
//! (use case) and the driven side (reader adapter) compose: the use case
//! depends on the trait, the composition root supplies the concrete
//! adapter.

use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;

use crate::error::application_error::ApplicationError;
use crate::ports::trajectory_reader::TrajectoryReader;
use crate::use_cases::trajectory_validation_failure::TrajectoryValidationFailure;
use crate::use_cases::validation_report::ValidationReport;

#[derive(Debug)]
pub struct ValidateTrajectoriesFromSourceUseCase<V, R>
where
    V: ActionContractValidator,
    R: TrajectoryReader,
{
    validator: V,
    reader: R,
}

impl<V, R> ValidateTrajectoriesFromSourceUseCase<V, R>
where
    V: ActionContractValidator,
    R: TrajectoryReader,
{
    pub fn new(validator: V, reader: R) -> Self {
        Self { validator, reader }
    }

    pub fn execute(&self) -> Result<ValidationReport, ApplicationError> {
        let mut report = ValidationReport::new();
        for trajectory in self.reader.read_all()? {
            match self.validator.validate(
                trajectory.target_action(),
                trajectory.about(),
                trajectory.mode(),
                trajectory.visible_state(),
            ) {
                Ok(()) => report.record_success(),
                Err(violations) => report.record_failure(TrajectoryValidationFailure::new(
                    trajectory.id().clone(),
                    violations,
                )),
            }
        }
        Ok(report)
    }
}
