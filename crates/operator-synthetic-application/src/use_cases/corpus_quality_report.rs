//! Report produced by corpus-quality evaluation.

use operator_synthetic_domain::quality::corpus_quality_violations::CorpusQualityViolations;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusQualityReport {
    violations: CorpusQualityViolations,
}

impl CorpusQualityReport {
    pub fn passed() -> Self {
        Self {
            violations: CorpusQualityViolations::new(),
        }
    }

    pub fn failed(violations: CorpusQualityViolations) -> Self {
        Self { violations }
    }

    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn violations(&self) -> &CorpusQualityViolations {
        &self.violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::contract::contract_violation::ContractViolation;
    use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;

    #[test]
    fn passed_report_has_no_violations() {
        let report = CorpusQualityReport::passed();
        assert!(report.is_valid());
        assert_eq!(report.violations().len(), 0);
    }

    #[test]
    fn failed_report_exposes_violations() {
        let mut violations = CorpusQualityViolations::new();
        violations.push(ContractViolation::new(
            ContractViolationCode::NoGoldAudit,
            "audit",
            "leak",
        ));
        let report = CorpusQualityReport::failed(violations);
        assert!(!report.is_valid());
        assert_eq!(report.violations().len(), 1);
    }
}
