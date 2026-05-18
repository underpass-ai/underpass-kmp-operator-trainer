use operator_shared_application::error::application_error::ApplicationError;
use thiserror::Error;

use crate::mappers::mapping_error::MappingError;

/// Infrastructure-side error. Wraps lower-level errors with enough context
/// for the adapter to translate into an `ApplicationError`.
#[derive(Debug, Error)]
pub enum InfraError {
    #[error("io error in {context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("serde_json error in {context}: {message}")]
    Serde {
        context: &'static str,
        message: String,
    },

    #[error(transparent)]
    Mapping(#[from] MappingError),
}

impl InfraError {
    pub fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    pub fn serde(context: &'static str, source: &serde_json::Error) -> Self {
        Self::Serde {
            context,
            message: source.to_string(),
        }
    }
}

impl From<InfraError> for ApplicationError {
    fn from(value: InfraError) -> Self {
        match value {
            InfraError::Mapping(MappingError::Domain(domain)) => ApplicationError::Domain(domain),
            other => ApplicationError::PortFailure {
                port: "shared-infra",
                message: other.to_string(),
            },
        }
    }
}
