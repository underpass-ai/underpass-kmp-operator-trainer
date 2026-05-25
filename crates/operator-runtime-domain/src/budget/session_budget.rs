use crate::error::runtime_domain_error::RuntimeDomainError;
use crate::error::runtime_domain_result::RuntimeDomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionBudget {
    calls_remaining: u32,
    tokens_remaining: u32,
}

impl SessionBudget {
    pub fn new(calls_remaining: u32, tokens_remaining: u32) -> Self {
        Self {
            calls_remaining,
            tokens_remaining,
        }
    }

    pub fn try_consume_call(self) -> RuntimeDomainResult<Self> {
        if self.calls_remaining == 0 {
            return Err(RuntimeDomainError::BudgetExhausted);
        }
        Ok(Self {
            calls_remaining: self.calls_remaining - 1,
            tokens_remaining: self.tokens_remaining,
        })
    }

    pub fn allows_call(&self) -> bool {
        self.calls_remaining > 0
    }

    pub fn calls_remaining(&self) -> u32 {
        self.calls_remaining
    }

    pub fn tokens_remaining(&self) -> u32 {
        self.tokens_remaining
    }
}
