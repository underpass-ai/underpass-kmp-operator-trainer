//! Port: serialise a slice of `TrainingTrajectory` values to whatever
//! medium the adapter targets and return a typed `DatasetWriteOutcome`
//! describing what landed (content hash, trajectory count, per-task-
//! family distribution).
//!
//! The use case never sees the bytes or the format; the adapter owns
//! both. JSONL SFT is the first concrete format; other formats (DPO,
//! raw dump, parquet) land as additional adapters behind this port.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::errors::dataset_write_error::DatasetWriteError;
use crate::ports::dataset_write_outcome::DatasetWriteOutcome;

pub trait DatasetWriter: std::fmt::Debug + Send + Sync {
    fn write(
        &self,
        trajectories: &[TrainingTrajectory],
    ) -> Result<DatasetWriteOutcome, DatasetWriteError>;
}
