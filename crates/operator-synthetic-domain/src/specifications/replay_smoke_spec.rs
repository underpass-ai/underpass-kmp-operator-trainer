//! Corpus spec: a replay smoke has been recorded and succeeded.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ReplaySmokeSpec;

impl ReplaySmokeSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ReplaySmokeSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        if !subject.audit().replay_smoke_recorded() {
            return Err(ContractViolation::new(
                ContractViolationCode::ReplaySmoke,
                "audit.replay_smoke_recorded",
                "replay smoke was not recorded",
            ));
        }
        let failures = subject.audit().replay_failures().as_usize();
        if failures == 0 {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::ReplaySmoke,
                "audit.replay_failures",
                format!("{failures} replay smoke rows failed"),
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
    fn accepts_recorded_replay_smoke() {
        ReplaySmokeSpec::new()
            .evaluate(&snapshot_with_audit(CorpusAuditSnapshot::clean()))
            .unwrap();
    }

    #[test]
    fn rejects_missing_or_failing_replay_smoke() {
        let missing = ReplaySmokeSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().without_replay_smoke(),
            ))
            .unwrap_err();
        assert_eq!(missing.code(), ContractViolationCode::ReplaySmoke);

        let failing = ReplaySmokeSpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_replay_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(failing.code(), ContractViolationCode::ReplaySmoke);
    }
}
