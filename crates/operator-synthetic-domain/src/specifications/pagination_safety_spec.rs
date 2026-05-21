//! Corpus spec: paginated navigation rows carry coherent page metadata.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct PaginationSafetySpec;

impl PaginationSafetySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for PaginationSafetySpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().pagination_safety_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::PaginationSafety,
                "audit.pagination_safety_failures",
                format!("{failures} rows had invalid page metadata"),
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
    fn accepts_clean_pagination_audit() {
        PaginationSafetySpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_pagination_failures() {
        let err = PaginationSafetySpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_pagination_safety_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::PaginationSafety);
    }
}
