//! Port for writing `TrainingTrajectory` aggregates to any sink. Adapters
//! in `operator-shared-infra` implement this trait. Encoding is the
//! adapter's responsibility.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::error::application_error::ApplicationResult;

pub trait TrajectoryWriter: std::fmt::Debug + Send + Sync {
    fn write(&mut self, trajectory: &TrainingTrajectory) -> ApplicationResult<()>;
}
