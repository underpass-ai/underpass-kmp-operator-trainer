use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeInfraError {
    #[error("failed to read TLS file {path}: {message}")]
    TlsFileRead { path: PathBuf, message: String },

    #[error("failed to build mTLS identity: {message}")]
    TlsIdentity { message: String },

    #[error("failed to build HTTP client: {message}")]
    HttpClient { message: String },

    #[error("client cert and client key must be provided together")]
    IncompleteMtlsConfig,
}
