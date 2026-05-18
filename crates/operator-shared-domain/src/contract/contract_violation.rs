//! `ContractViolation` is the typed result of a failed specification. It
//! carries enough information for a dataset/replay/evaluation report to
//! reproduce the failure without re-running validation.

use crate::contract::contract_violation_code::ContractViolationCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractViolation {
    code: ContractViolationCode,
    field: String,
    message: String,
}

impl ContractViolation {
    pub fn new(
        code: ContractViolationCode,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> ContractViolationCode {
        self.code
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
