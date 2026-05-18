use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

/// Application-layer error. Carries either a typed domain error or a port
/// failure summary. Infrastructure errors must be translated into one of
/// these variants by the adapter; raw `std::io::Error` or `serde_json::Error`
/// never cross this boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplicationError {
    #[error("port '{port}' failed: {message}")]
    PortFailure { port: &'static str, message: String },

    #[error(transparent)]
    Domain(#[from] DomainError),
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;
