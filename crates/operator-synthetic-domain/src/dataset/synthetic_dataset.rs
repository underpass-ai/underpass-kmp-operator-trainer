//! Result of a single synthetic generation run: the dataset identifier
//! and the trajectories collected across every case in the blueprint.

use operator_shared_domain::ids::dataset_id::DatasetId;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticDataset {
    dataset_id: DatasetId,
    trajectories: Vec<TrainingTrajectory>,
}

impl SyntheticDataset {
    pub fn new(
        dataset_id: DatasetId,
        trajectories: Vec<TrainingTrajectory>,
    ) -> SyntheticDomainResult<Self> {
        if trajectories.is_empty() {
            return Err(SyntheticDomainError::EmptyDataset);
        }
        Ok(Self {
            dataset_id,
            trajectories,
        })
    }

    pub fn dataset_id(&self) -> &DatasetId {
        &self.dataset_id
    }

    pub fn trajectories(&self) -> &[TrainingTrajectory] {
        &self.trajectories
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_trajectories() {
        let err =
            SyntheticDataset::new(DatasetId::parse("dataset:empty").unwrap(), vec![]).unwrap_err();
        assert!(matches!(err, SyntheticDomainError::EmptyDataset));
    }
}
