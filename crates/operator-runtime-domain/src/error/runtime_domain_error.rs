use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuntimeDomainError {
    #[error("session budget has no calls remaining")]
    BudgetExhausted,

    #[error("allowed tools were built for mode '{actual}', expected '{expected}'")]
    AllowedToolsModeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    #[error("tool {tool} is not allowed in runtime mode '{mode}'")]
    ToolNotAllowedInRuntimeMode {
        mode: &'static str,
        tool: KernelTool,
    },

    #[error(transparent)]
    Shared(#[from] DomainError),
}
