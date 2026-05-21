//! Ordered corpus-quality violations returned by the composite validator.

use operator_shared_domain::contract::contract_violation::ContractViolation;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CorpusQualityViolations {
    items: Vec<ContractViolation>,
}

impl CorpusQualityViolations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, violation: ContractViolation) {
        self.items.push(violation);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;

    #[test]
    fn collects_violations_in_order() {
        let mut violations = CorpusQualityViolations::new();
        violations.push(ContractViolation::new(
            ContractViolationCode::SchemaParse,
            "schema",
            "bad",
        ));
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations.as_slice()[0].code(),
            ContractViolationCode::SchemaParse
        );
    }
}
