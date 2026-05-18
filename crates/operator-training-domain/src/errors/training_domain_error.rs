//! Domain-level errors for the training context. Wraps the shared
//! domain error transparently and adds training-specific variants.

use operator_shared_domain::error::domain_error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrainingDomainError {
    #[error(transparent)]
    Shared(#[from] DomainError),

    #[error("readiness report cannot be built from an empty gate list")]
    EmptyReadinessReport,

    #[error(
        "training run cannot launch: readiness report failed {failed} of {total} gates: {summary}"
    )]
    NotReady {
        failed: usize,
        total: usize,
        summary: String,
    },

    #[error("manifest run_id '{manifest}' does not match training run id '{run}'")]
    ManifestRunIdMismatch { manifest: String, run: String },

    #[error("task family '{family}' is declared twice in the dataset distribution")]
    DuplicateTaskFamilyInDistribution { family: String },

    #[error(
        "dataset provenance trajectory_count {trajectory_count} disagrees with task family distribution total {distribution_total}"
    )]
    DistributionTotalMismatch {
        trajectory_count: usize,
        distribution_total: usize,
    },
}
