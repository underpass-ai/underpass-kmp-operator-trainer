//! Corpus spec: about/scope decisions are explicit and safe.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ScopeSafetySpec;

impl ScopeSafetySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ScopeSafetySpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().scope_safety_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::ScopeSafety,
                "audit.scope_safety_failures",
                format!("{failures} rows used unsafe scope"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_with_about_not_in_known, snapshot_with_audit,
    };
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_clean_scope_audit() {
        ScopeSafetySpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_scope_failures() {
        let err = ScopeSafetySpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_scope_safety_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ScopeSafety);
    }

    #[test]
    fn scope_safety_spec_accepts_clean_corpus() {
        ScopeSafetySpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn scope_safety_spec_rejects_about_not_in_known() {
        let err = ScopeSafetySpec::new()
            .evaluate(&corpus_with_about_not_in_known())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ScopeSafety);
        assert!(err.field().contains("scope_safety"));
    }
}
