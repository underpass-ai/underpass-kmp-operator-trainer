//! Corpus spec: every KMP/MCP tool appears in target actions.

use std::collections::BTreeSet;

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;
use operator_shared_domain::tool::kernel_tool::KernelTool;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct ContractCoverageSpec;

impl ContractCoverageSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for ContractCoverageSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let covered: BTreeSet<KernelTool> = subject
            .dataset()
            .trajectories()
            .iter()
            .filter_map(|trajectory| trajectory.target_action().tool())
            .collect();
        let missing: Vec<&'static str> = KernelTool::ALL
            .iter()
            .copied()
            .filter(|tool| !covered.contains(tool))
            .map(KernelTool::as_str)
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ContractViolation::new(
                ContractViolationCode::ContractCoverage,
                "dataset.trajectories.target_action.tool",
                format!("missing tools: {}", missing.join(",")),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::test_support::{
        clean_corpus_snapshot, corpus_missing_kernel_forward, full_quality_snapshot,
        inspect_snapshot_without_split,
    };

    #[test]
    fn accepts_full_tool_coverage() {
        ContractCoverageSpec::new()
            .evaluate(&full_quality_snapshot())
            .unwrap();
    }

    #[test]
    fn rejects_missing_tool_coverage() {
        let err = ContractCoverageSpec::new()
            .evaluate(&inspect_snapshot_without_split())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ContractCoverage);
    }

    #[test]
    fn contract_coverage_spec_accepts_clean_corpus() {
        ContractCoverageSpec::new()
            .evaluate(&clean_corpus_snapshot())
            .unwrap();
    }

    #[test]
    fn contract_coverage_spec_rejects_missing_kernel_forward() {
        let err = ContractCoverageSpec::new()
            .evaluate(&corpus_missing_kernel_forward())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ContractCoverage);
        assert!(err.field().contains("target_action.tool"));
    }
}
