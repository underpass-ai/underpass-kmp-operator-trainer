//! Corpus spec: target refs, cursors and dimensions are visible.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ReferenceSafetySpec;

impl ReferenceSafetySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ReferenceSafetySpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().reference_safety_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::ReferenceSafety,
                "audit.reference_safety_failures",
                format!("{failures} rows referenced non-visible state"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::snapshot_with_audit;
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_clean_reference_audit() {
        ReferenceSafetySpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_reference_failures() {
        let err = ReferenceSafetySpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_reference_safety_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ReferenceSafety);
    }
}
