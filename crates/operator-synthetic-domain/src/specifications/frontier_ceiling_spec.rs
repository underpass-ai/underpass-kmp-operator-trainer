//! Corpus spec: the eval split has a frontier-model ceiling baseline.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct FrontierCeilingSpec;

impl FrontierCeilingSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for FrontierCeilingSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        if subject.audit().frontier_ceiling_recorded() {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::FrontierCeiling,
                "audit.frontier_ceiling_recorded",
                "frontier ceiling was not recorded",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::snapshot_with_audit;

    #[test]
    fn accepts_recorded_frontier_ceiling() {
        FrontierCeilingSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_missing_frontier_ceiling() {
        let err = FrontierCeilingSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().without_frontier_ceiling(),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::FrontierCeiling);
    }
}
