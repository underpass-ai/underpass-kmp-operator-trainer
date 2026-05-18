use crate::errors::training_application_error::TrainingApplicationError;

pub type TrainingApplicationResult<T> = Result<T, TrainingApplicationError>;
