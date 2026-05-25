use operator_shared_domain::contract::contract_violations::ContractViolations;

use crate::session::observation_error_code::ObservationErrorCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeClass {
    Completed,
    Escalated,
    BudgetExhausted,
    ContractViolation { violations: ContractViolations },
    McpExecutionFailure { code: ObservationErrorCode },
}

impl OutcomeClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Escalated => "escalated",
            Self::BudgetExhausted => "budget_exhausted",
            Self::ContractViolation { .. } => "contract_violation",
            Self::McpExecutionFailure { .. } => "mcp_execution_failure",
        }
    }
}
