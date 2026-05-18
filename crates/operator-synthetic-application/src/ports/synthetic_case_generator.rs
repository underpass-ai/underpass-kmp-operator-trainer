//! Port: generate `TrainingTrajectory` instances for a single
//! `SyntheticCaseSpec`. Adapters in `operator-synthetic-infra` implement
//! this trait; the use case is generator-agnostic.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;

use crate::error::generate_synthetic_case_error::GenerateSyntheticCaseError;

pub trait SyntheticCaseGenerator: std::fmt::Debug + Send + Sync {
    fn generate(
        &self,
        spec: &SyntheticCaseSpec,
    ) -> Result<Vec<TrainingTrajectory>, GenerateSyntheticCaseError>;
}
