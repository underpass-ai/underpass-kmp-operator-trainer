//! Corpus spec: each target tool belongs to the row operator mode.

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ModeSafetySpec;

impl ModeSafetySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ModeSafetySpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let failures = subject.audit().mode_safety_failures().as_usize();
        if failures > 0 {
            return Err(ContractViolation::new(
                ContractViolationCode::ModeSafety,
                "audit.mode_safety_failures",
                format!("{failures} rows used tools outside mode"),
            ));
        }
        for trajectory in subject.dataset().trajectories() {
            let canonical = AllowedTools::for_mode(trajectory.mode());
            if canonical.as_slice() != trajectory.allowed_tools().as_slice() {
                return Err(ContractViolation::new(
                    ContractViolationCode::ModeSafety,
                    "trajectory.allowed_tools",
                    "allowed tools do not match row mode",
                ));
            }
            if let Some(tool) = trajectory.target_action().tool()
                && !trajectory.allowed_tools().contains(tool)
            {
                return Err(ContractViolation::new(
                    ContractViolationCode::ModeSafety,
                    "trajectory.target_action.tool",
                    format!(
                        "{} is not allowed in {}",
                        tool.as_str(),
                        trajectory.mode().as_str()
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_with_ingest_in_read_mode, full_quality_snapshot,
        snapshot_with_audit,
    };
    use operator_shared_domain::value_objects::example_count::ExampleCount;

    #[test]
    fn accepts_canonical_mode_safe_rows() {
        ModeSafetySpec::new()
            .evaluate(&full_quality_snapshot())
            .unwrap();
    }

    #[test]
    fn has_stable_debug_name_for_reports() {
        assert!(format!("{:?}", ModeSafetySpec::new()).contains("ModeSafetySpec"));
    }

    #[test]
    fn rejects_mode_safety_failures() {
        let err = ModeSafetySpec::new()
            .evaluate(&snapshot_with_audit(
                CorpusAuditSnapshot::clean().with_mode_safety_failures(ExampleCount::new(1)),
            ))
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ModeSafety);
    }

    #[test]
    fn mode_safety_spec_accepts_clean_corpus() {
        ModeSafetySpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn mode_safety_spec_rejects_ingest_in_read_mode() {
        let err = ModeSafetySpec::new()
            .evaluate(&corpus_with_ingest_in_read_mode())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ModeSafety);
        assert!(err.field().contains("mode_safety"));
    }
}
