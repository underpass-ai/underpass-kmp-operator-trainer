use crate::error::domain_error::DomainError;

pub type DomainResult<T> = Result<T, DomainError>;
