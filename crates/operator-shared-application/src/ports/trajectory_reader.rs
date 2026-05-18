//! Port for reading `TrainingTrajectory` aggregates from any source.
//! Adapters in `operator-shared-infra` implement this trait. The port
//! returns domain types; format-specific decoding is the adapter's job.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::error::application_error::ApplicationResult;

pub trait TrajectoryReader: std::fmt::Debug + Send + Sync {
    fn read_all(&self) -> ApplicationResult<Vec<TrainingTrajectory>>;
}
