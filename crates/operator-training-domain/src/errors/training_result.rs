use crate::errors::training_domain_error::TrainingDomainError;

pub type TrainingResult<T> = Result<T, TrainingDomainError>;
