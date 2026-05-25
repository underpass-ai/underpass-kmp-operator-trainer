use crate::error::runtime_domain_error::RuntimeDomainError;

pub type RuntimeDomainResult<T> = Result<T, RuntimeDomainError>;
