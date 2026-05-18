use crate::error::replay_domain_error::ReplayDomainError;

pub type ReplayDomainResult<T> = Result<T, ReplayDomainError>;
