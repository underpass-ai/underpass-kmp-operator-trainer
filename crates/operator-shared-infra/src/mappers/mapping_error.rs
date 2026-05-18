use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

/// Errors raised when translating between DTOs and domain types. Domain
/// invariant violations are surfaced verbatim; structural problems on the
/// wire (missing field, unsupported discriminator) get their own variant.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MappingError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("unsupported discriminator '{value}' for {context}")]
    UnsupportedDiscriminator {
        context: &'static str,
        value: String,
    },

    #[error("tool '{tool}' arguments shape is invalid: {message}")]
    InvalidToolArguments { tool: &'static str, message: String },
}
