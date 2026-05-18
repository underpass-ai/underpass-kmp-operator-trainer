//! Port: read a stream of `TrainingTrajectory` values from an
//! adapter-defined source (filesystem, network, synthetic-run
//! handoff, in-memory test fixture, …). The use case never sees the
//! bytes; only the typed trajectories.
//!
//! Adapters implementing this trait live in `operator-training-infra`.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::errors::trajectory_source_error::TrajectorySourceError;

pub trait TrajectorySource: std::fmt::Debug + Send + Sync {
    /// Load every trajectory exposed by this source. Returns the
    /// vector in source order; the use case is responsible for any
    /// downstream re-ordering.
    fn fetch_all(&self) -> Result<Vec<TrainingTrajectory>, TrajectorySourceError>;
}
