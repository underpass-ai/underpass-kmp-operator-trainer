//! Provenance of a training dataset: where it came from, its content
//! hash, the number of trajectories it contains, and the distribution
//! across task families.
//!
//! ## Distribution contract
//!
//! The distribution is intentionally allowed to be **empty**, meaning
//! "the caller has not computed per-family counts for this dataset".
//! When the distribution is non-empty, its total **must** equal
//! `trajectory_count` — the constructor refuses to build a value that
//! contradicts itself.
//!
//! Callers that want a "no distribution" provenance pass
//! `TaskFamilyDistribution::new(vec![])`. Callers that want a
//! "distribution known and consistent" provenance pass a non-empty
//! distribution whose `total()` matches `trajectory_count`. There is
//! no third state.
//!
//! Use `has_distribution()` to discriminate at consumer sites instead
//! of inspecting `distribution().family_count() > 0` by hand.

use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::errors::training_domain_error::TrainingDomainError;
use crate::errors::training_result::TrainingResult;
use crate::provenance::content_hash::ContentHash;
use crate::provenance::dataset_source::DatasetSource;
use crate::provenance::task_family_distribution::TaskFamilyDistribution;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetProvenance {
    source: DatasetSource,
    content_hash: ContentHash,
    trajectory_count: PositiveCount,
    distribution: TaskFamilyDistribution,
}

impl DatasetProvenance {
    pub fn new(
        source: DatasetSource,
        content_hash: ContentHash,
        trajectory_count: PositiveCount,
        distribution: TaskFamilyDistribution,
    ) -> TrainingResult<Self> {
        let total = distribution.total();
        // Allow an empty distribution (caller may have skipped per-
        // family counts) but require that any non-empty distribution
        // total matches the declared trajectory count.
        if distribution.family_count() > 0 && total != trajectory_count.as_usize() {
            return Err(TrainingDomainError::DistributionTotalMismatch {
                trajectory_count: trajectory_count.as_usize(),
                distribution_total: total,
            });
        }
        Ok(Self {
            source,
            content_hash,
            trajectory_count,
            distribution,
        })
    }

    pub fn source(&self) -> &DatasetSource {
        &self.source
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

    /// `true` iff the per-task-family distribution has been computed
    /// for this dataset. When `false`, `distribution()` returns an
    /// empty (but legal) value and gates that depend on coverage
    /// (e.g., `MinimumTaskFamilyCoverage`) will report zero.
    pub fn has_distribution(&self) -> bool {
        self.distribution.family_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
    use operator_shared_domain::value_objects::task_family::TaskFamily;

    fn entry(family: &str, count: usize) -> TaskFamilyDistributionEntry {
        TaskFamilyDistributionEntry::new(
            TaskFamily::parse(family).unwrap(),
            PositiveCount::parse(count, "test").unwrap(),
        )
    }

    fn provenance(
        trajectory_count: usize,
        distribution: TaskFamilyDistribution,
    ) -> TrainingResult<DatasetProvenance> {
        DatasetProvenance::new(
            DatasetSource::parse("synthetic-run:test").unwrap(),
            ContentHash::parse("sha256:abc").unwrap(),
            PositiveCount::parse(trajectory_count, "trajectory_count").unwrap(),
            distribution,
        )
    }

    #[test]
    fn refuses_distribution_total_mismatch() {
        let distribution =
            TaskFamilyDistribution::new(vec![entry("read.inspect", 3), entry("read.ask", 2)])
                .unwrap();
        let err = provenance(10, distribution).unwrap_err();
        assert!(matches!(
            err,
            TrainingDomainError::DistributionTotalMismatch {
                trajectory_count: 10,
                distribution_total: 5,
            }
        ));
    }

    #[test]
    fn allows_empty_distribution_without_total_check() {
        let distribution = TaskFamilyDistribution::new(vec![]).unwrap();
        let ok = provenance(10, distribution).expect("empty distribution must be allowed");
        assert_eq!(ok.trajectory_count().as_usize(), 10);
        assert_eq!(ok.distribution().family_count(), 0);
        assert!(
            !ok.has_distribution(),
            "empty distribution means has_distribution() is false"
        );
    }

    #[test]
    fn has_distribution_is_true_when_non_empty() {
        let distribution = TaskFamilyDistribution::new(vec![entry("read.inspect", 10)]).unwrap();
        let ok = provenance(10, distribution).unwrap();
        assert!(ok.has_distribution());
    }

    #[test]
    fn accepts_consistent_provenance() {
        let distribution =
            TaskFamilyDistribution::new(vec![entry("read.inspect", 6), entry("read.ask", 4)])
                .unwrap();
        let ok = provenance(10, distribution).expect("consistent provenance must build");
        assert_eq!(ok.source().as_str(), "synthetic-run:test");
        assert_eq!(ok.content_hash().as_str(), "sha256:abc");
        assert_eq!(ok.distribution().total(), 10);
    }
}
