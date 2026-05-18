use crate::contract::contract_violation::ContractViolation;

/// Aggregated, ordered collection of `ContractViolation`. Order matches the
/// order in which specifications were evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractViolations {
    items: Vec<ContractViolation>,
}

impl ContractViolations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, violation: ContractViolation) {
        self.items.push(violation);
    }

    pub fn extend(&mut self, other: ContractViolations) {
        self.items.extend(other.items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn as_slice(&self) -> &[ContractViolation] {
        &self.items
    }

    pub fn into_inner(self) -> Vec<ContractViolation> {
        self.items
    }
}
