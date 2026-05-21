//! Corpus spec: model-facing prompts do not leak gold labels.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct NoGoldAuditSpec;

impl NoGoldAuditSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for NoGoldAuditSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let findings = subject.audit().gold_leak_findings().as_usize();
        if findings == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::NoGoldAudit,
                "audit.gold_leak_findings",
                format!("{findings} gold leaks found"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_with_target_action_leaked_in_system_prompt,
        snapshot_with_audit,
    };
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_clean_no_gold_audit() {
        NoGoldAuditSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_gold_leak_findings() {
        let err = NoGoldAuditSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_gold_leak_findings(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::NoGoldAudit);
    }

    #[test]
    fn no_gold_audit_spec_accepts_clean_corpus() {
        NoGoldAuditSpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn no_gold_audit_spec_rejects_target_action_leaked_in_system_prompt() {
        let err = NoGoldAuditSpec::new()
            .evaluate(&corpus_with_target_action_leaked_in_system_prompt())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::NoGoldAudit);
        assert!(err.field().contains("gold_leak"));
    }
}
