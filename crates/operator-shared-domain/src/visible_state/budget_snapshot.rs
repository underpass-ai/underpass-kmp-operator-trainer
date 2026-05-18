//! Snapshot of the budget the operator sees when deciding the next action.
//! "Unbounded" is an explicit constructor (`unbounded()`) rather than a
//! `None` smuggled into the call sites.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetSnapshot {
    calls_remaining: BudgetField,
    tokens_remaining: BudgetField,
}

impl BudgetSnapshot {
    pub fn bounded(calls_remaining: usize, tokens_remaining: usize) -> Self {
        Self {
            calls_remaining: BudgetField::Bounded(calls_remaining),
            tokens_remaining: BudgetField::Bounded(tokens_remaining),
        }
    }

    pub fn unbounded() -> Self {
        Self {
            calls_remaining: BudgetField::Unbounded,
            tokens_remaining: BudgetField::Unbounded,
        }
    }

    #[must_use]
    pub fn with_calls_remaining(self, value: usize) -> Self {
        Self {
            calls_remaining: BudgetField::Bounded(value),
            ..self
        }
    }

    #[must_use]
    pub fn with_tokens_remaining(self, value: usize) -> Self {
        Self {
            tokens_remaining: BudgetField::Bounded(value),
            ..self
        }
    }

    pub fn calls_remaining(self) -> BudgetField {
        self.calls_remaining
    }

    pub fn tokens_remaining(self) -> BudgetField {
        self.tokens_remaining
    }

    pub fn allows_another_call(self) -> bool {
        match self.calls_remaining {
            BudgetField::Unbounded => true,
            BudgetField::Bounded(value) => value > 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    Unbounded,
    Bounded(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_always_allows_another_call() {
        assert!(BudgetSnapshot::unbounded().allows_another_call());
    }

    #[test]
    fn bounded_zero_calls_disallows_another_call() {
        assert!(!BudgetSnapshot::bounded(0, 100).allows_another_call());
    }

    #[test]
    fn bounded_positive_calls_allows_another_call() {
        assert!(BudgetSnapshot::bounded(1, 100).allows_another_call());
    }
}
