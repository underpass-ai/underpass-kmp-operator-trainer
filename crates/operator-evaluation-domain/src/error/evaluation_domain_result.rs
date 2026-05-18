use crate::error::evaluation_domain_error::EvaluationDomainError;

pub type EvaluationDomainResult<T> = Result<T, EvaluationDomainError>;
