use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryValidationFailure {
    trajectory_id: TrainingTrajectoryId,
    violations: ContractViolations,
}

impl TrajectoryValidationFailure {
    pub fn new(trajectory_id: TrainingTrajectoryId, violations: ContractViolations) -> Self {
        Self {
            trajectory_id,
            violations,
        }
    }

    pub fn trajectory_id(&self) -> &TrainingTrajectoryId {
        &self.trajectory_id
    }

    pub fn violations(&self) -> &ContractViolations {
        &self.violations
    }
}
