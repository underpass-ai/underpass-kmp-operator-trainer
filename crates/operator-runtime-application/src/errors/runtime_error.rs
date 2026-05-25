use operator_runtime_domain::error::runtime_domain_error::RuntimeDomainError;
use thiserror::Error;

use crate::errors::mcp_executor_error::McpExecutorError;
use crate::errors::operator_policy_error::OperatorPolicyError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to build runtime calibration subject: {message}")]
    SubjectBuild { message: String },

    #[error(transparent)]
    Policy(#[from] OperatorPolicyError),

    #[error(transparent)]
    Mcp(#[from] McpExecutorError),

    #[error(transparent)]
    Domain(#[from] RuntimeDomainError),
}
