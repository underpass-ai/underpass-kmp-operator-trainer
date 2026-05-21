//! Corpus spec: every source row parsed as a trajectory DTO.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct SchemaParseSpec;

impl SchemaParseSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for SchemaParseSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().schema_parse_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::SchemaParse,
                "audit.schema_parse_failures",
                format!("{failures} rows failed schema parsing"),
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
    fn accepts_clean_schema_parse_audit() {
        SchemaParseSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_schema_parse_failures() {
        let err = SchemaParseSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_schema_parse_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::SchemaParse);
    }
}
