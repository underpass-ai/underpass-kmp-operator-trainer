//! Corpus spec: duplicate decision rows are bounded and reported.

use std::collections::BTreeSet;

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct DuplicateAuditSpec;

impl DuplicateAuditSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for DuplicateAuditSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let mut seen = BTreeSet::new();
        for trajectory in subject.dataset().trajectories() {
            if !seen.insert(trajectory.step_id().as_str()) {
                return Err(ContractViolation::new(
                    ContractViolationCode::DuplicateAudit,
                    "dataset.trajectories.step_id",
                    format!("duplicate step_id {}", trajectory.step_id().as_str()),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::test_support::{duplicate_step_snapshot, full_quality_snapshot};

    #[test]
    fn accepts_unique_step_ids() {
        DuplicateAuditSpec::new()
            .evaluate(&full_quality_snapshot())
            .unwrap();
    }

    #[test]
    fn rejects_duplicate_step_ids() {
        let err = DuplicateAuditSpec::new()
            .evaluate(&duplicate_step_snapshot())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::DuplicateAudit);
    }
}
