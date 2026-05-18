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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::contract_violation_code::ContractViolationCode;

    fn sample(message: &str) -> ContractViolation {
        ContractViolation::new(ContractViolationCode::BudgetExhausted, "f", message)
    }

    #[test]
    fn new_is_empty() {
        let v = ContractViolations::new();
        assert!(v.is_empty());
        assert_eq!(v.len(), 0);
        assert!(v.as_slice().is_empty());
    }

    #[test]
    fn push_then_inspect() {
        let mut v = ContractViolations::new();
        v.push(sample("a"));
        assert_eq!(v.len(), 1);
        assert_eq!(v.as_slice()[0].message(), "a");
    }

    #[test]
    fn extend_merges_two_collections() {
        let mut a = ContractViolations::new();
        a.push(sample("a"));
        let mut b = ContractViolations::new();
        b.push(sample("b"));
        a.extend(b);
        assert_eq!(a.len(), 2);
        assert_eq!(a.into_inner().len(), 2);
    }
}
