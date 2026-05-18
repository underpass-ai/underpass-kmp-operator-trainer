use crate::error::synthetic_domain_error::SyntheticDomainError;

pub type SyntheticDomainResult<T> = Result<T, SyntheticDomainError>;
