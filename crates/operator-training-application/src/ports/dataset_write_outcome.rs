//! Typed value object returned by `DatasetWriter::write`. Carries the
//! content hash of the written bytes, the trajectory count that
//! actually landed, and the per-task-family distribution computed
//! while writing. The use case builds a `DatasetProvenance` from this
//! triple; the writer never sees `DatasetProvenance` itself.

use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_training_domain::provenance::content_hash::ContentHash;
use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetWriteOutcome {
    content_hash: ContentHash,
    trajectory_count: PositiveCount,
    distribution: TaskFamilyDistribution,
}

impl DatasetWriteOutcome {
    pub fn new(
        content_hash: ContentHash,
        trajectory_count: PositiveCount,
        distribution: TaskFamilyDistribution,
    ) -> Self {
        Self {
            content_hash,
            trajectory_count,
            distribution,
        }
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    pub fn trajectory_count(&self) -> PositiveCount {
        self.trajectory_count
    }

    pub fn distribution(&self) -> &TaskFamilyDistribution {
        &self.distribution
    }
}
