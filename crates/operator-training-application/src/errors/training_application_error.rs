//! Umbrella error for the training application layer. Wraps the
//! domain error and every port-specific error transparently so the use
//! cases can `?`-bubble them without losing the originating variant.

use operator_training_domain::errors::training_domain_error::TrainingDomainError;
use thiserror::Error;

use crate::errors::dataset_write_error::DatasetWriteError;
use crate::errors::manifest_write_error::ManifestWriteError;
use crate::errors::policy_evaluator_error::PolicyEvaluatorError;
use crate::errors::predictions_read_error::PredictionsReadError;
use crate::errors::predictor_error::PredictorError;
use crate::errors::trainer_invoker_error::TrainerInvokerError;
use crate::errors::trajectory_source_error::TrajectorySourceError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrainingApplicationError {
    #[error(transparent)]
    Domain(#[from] TrainingDomainError),

    #[error(transparent)]
    TrajectorySource(#[from] TrajectorySourceError),

    #[error(transparent)]
    DatasetWrite(#[from] DatasetWriteError),

    #[error(transparent)]
    ManifestWrite(#[from] ManifestWriteError),

    #[error(transparent)]
    TrainerInvoker(#[from] TrainerInvokerError),

    #[error(transparent)]
    Predictor(#[from] PredictorError),

    #[error(transparent)]
    PredictionsRead(#[from] PredictionsReadError),

    #[error(transparent)]
    PolicyEvaluator(#[from] PolicyEvaluatorError),
}
