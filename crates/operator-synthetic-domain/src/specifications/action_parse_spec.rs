//! Corpus spec: every target action parsed as an action DTO/domain value.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ActionParseSpec;

impl ActionParseSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ActionParseSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().action_parse_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::ActionParse,
                "audit.action_parse_failures",
                format!("{failures} target actions failed parsing"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_with_invalid_action_target, snapshot_with_audit,
    };
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_clean_action_parse_audit() {
        ActionParseSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_action_parse_failures() {
        let err = ActionParseSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_action_parse_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ActionParse);
    }

    #[test]
    fn action_parse_spec_accepts_clean_corpus() {
        ActionParseSpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn action_parse_spec_rejects_invalid_action_target() {
        let err = ActionParseSpec::new()
            .evaluate(&corpus_with_invalid_action_target())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ActionParse);
        assert!(err.field().contains("action_parse"));
    }
}
