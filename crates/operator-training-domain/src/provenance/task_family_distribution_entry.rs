//! One row of `TaskFamilyDistribution`: the family identifier and the
//! count of trajectories tagged with that family. Refuses to model a
//! zero-count entry — collapse those at the caller level.

use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFamilyDistributionEntry {
    family: TaskFamily,
    count: PositiveCount,
}

impl TaskFamilyDistributionEntry {
    pub fn new(family: TaskFamily, count: PositiveCount) -> Self {
        Self { family, count }
    }

    pub fn family(&self) -> &TaskFamily {
        &self.family
    }

    pub fn count(&self) -> PositiveCount {
        self.count
    }
}
