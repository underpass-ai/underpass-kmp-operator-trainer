//! Corpus spec: rich write rows prove their read-before-write context.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct WriteProofSpec;

impl WriteProofSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for WriteProofSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().write_proof_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::WriteProof,
                "audit.write_proof_failures",
                format!("{failures} write rows lacked proof"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_with_write_lacking_read_before_write, snapshot_with_audit,
    };
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_clean_write_proof_audit() {
        WriteProofSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_write_proof_failures() {
        let err = WriteProofSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_write_proof_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::WriteProof);
    }

    #[test]
    fn write_proof_spec_accepts_clean_corpus() {
        WriteProofSpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn write_proof_spec_rejects_write_lacking_read_before_write() {
        let err = WriteProofSpec::new()
            .evaluate(&corpus_with_write_lacking_read_before_write())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::WriteProof);
        assert!(err.field().contains("write_proof"));
    }
}
