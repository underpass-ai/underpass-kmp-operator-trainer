//! Distribution of trajectories across task families inside a
//! `DatasetProvenance`. The collection refuses to be built with
//! duplicate families or empty entries.

use std::collections::BTreeSet;

use crate::errors::training_domain_error::TrainingDomainError;
use crate::errors::training_result::TrainingResult;
use crate::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFamilyDistribution {
    entries: Vec<TaskFamilyDistributionEntry>,
}

impl TaskFamilyDistribution {
    /// Build a distribution from a list of `(TaskFamily, PositiveCount)`
    /// entries. Refuses duplicates so the manifest is unambiguous.
    pub fn new(entries: Vec<TaskFamilyDistributionEntry>) -> TrainingResult<Self> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for entry in &entries {
            let key = entry.family().as_str().to_string();
            if !seen.insert(key.clone()) {
                return Err(TrainingDomainError::DuplicateTaskFamilyInDistribution { family: key });
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[TaskFamilyDistributionEntry] {
        &self.entries
    }

    /// Total number of trajectories described by this distribution.
    pub fn total(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.count().as_usize())
            .sum::<usize>()
    }

    /// Number of distinct task families covered.
    pub fn family_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::task_family::TaskFamily;

    fn entry(family: &str, count: usize) -> TaskFamilyDistributionEntry {
        TaskFamilyDistributionEntry::new(
            TaskFamily::parse(family).unwrap(),
            PositiveCount::parse(count, "test").unwrap(),
        )
    }

    #[test]
    fn refuses_duplicate_families() {
        let err =
            TaskFamilyDistribution::new(vec![entry("read.inspect", 3), entry("read.inspect", 1)])
                .unwrap_err();
        assert!(matches!(
            err,
            TrainingDomainError::DuplicateTaskFamilyInDistribution { .. }
        ));
    }

    #[test]
    fn aggregates_total_and_family_count() {
        let d = TaskFamilyDistribution::new(vec![
            entry("read.inspect", 3),
            entry("read.ask", 2),
            entry("read.wake", 4),
        ])
        .unwrap();
        assert_eq!(d.family_count(), 3);
        assert_eq!(d.total(), 9);
    }

    #[test]
    fn empty_distribution_is_legal() {
        let d = TaskFamilyDistribution::new(vec![]).unwrap();
        assert_eq!(d.family_count(), 0);
        assert_eq!(d.total(), 0);
    }
}
