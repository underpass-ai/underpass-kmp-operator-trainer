//! Blueprint: the set of `SyntheticCaseSpec`s that a single dataset
//! generation run must cover. The constructor refuses to build an empty
//! blueprint or one with duplicate case identifiers.

use std::collections::BTreeSet;

use operator_shared_domain::ids::dataset_id::DatasetId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::case::synthetic_case_spec::SyntheticCaseSpec;
use crate::case::synthetic_generation_target::SyntheticGenerationTarget;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticDatasetBlueprint {
    dataset_id: DatasetId,
    cases: Vec<SyntheticCaseSpec>,
}

impl SyntheticDatasetBlueprint {
    pub fn new(
        dataset_id: DatasetId,
        cases: Vec<SyntheticCaseSpec>,
    ) -> SyntheticDomainResult<Self> {
        if cases.is_empty() {
            return Err(SyntheticDomainError::EmptyBlueprint);
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for case in &cases {
            if !seen.insert(case.case_id().as_str()) {
                return Err(SyntheticDomainError::DuplicateCase {
                    case_id: case.case_id().as_str().to_string(),
                });
            }
        }
        Ok(Self { dataset_id, cases })
    }

    /// Factory: produces one canonical case per `KmpMcpCapability` variant,
    /// each with the same `minimum_examples`. Useful for production
    /// blueprints that demand full coverage by default.
    pub fn for_all_capabilities(
        dataset_id: DatasetId,
        minimum_examples_per_capability: PositiveCount,
    ) -> SyntheticDomainResult<Self> {
        let mut cases = Vec::with_capacity(KmpMcpCapability::ALL.len());
        for capability in KmpMcpCapability::ALL {
            let case_id = operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId::parse(
                format!("kmp_mcp:{}", capability.name()),
            )?;
            cases.push(SyntheticCaseSpec::new(
                case_id,
                capability,
                minimum_examples_per_capability,
            ));
        }
        Self::new(dataset_id, cases)
    }

    /// Factory: produces one canonical case per synthetic generation target.
    /// This is the 12-target surface used by realistic Operator corpora:
    /// all 10 KMP tools plus `stop` and `escalate`.
    pub fn for_all_generation_targets(
        dataset_id: DatasetId,
        minimum_examples_per_target: PositiveCount,
    ) -> SyntheticDomainResult<Self> {
        let mut cases = Vec::with_capacity(SyntheticGenerationTarget::ALL.len());
        for target in SyntheticGenerationTarget::ALL {
            let case_id = operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId::parse(
                format!("operator_action:{}", target.name()),
            )?;
            cases.push(SyntheticCaseSpec::for_target(
                case_id,
                target,
                minimum_examples_per_target,
            ));
        }
        Self::new(dataset_id, cases)
    }

    pub fn dataset_id(&self) -> &DatasetId {
        &self.dataset_id
    }

    pub fn cases(&self) -> &[SyntheticCaseSpec] {
        &self.cases
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;

    fn case(id: &str, capability: KmpMcpCapability) -> SyntheticCaseSpec {
        SyntheticCaseSpec::new(
            SyntheticCaseId::parse(id).unwrap(),
            capability,
            PositiveCount::parse(1, "minimum").unwrap(),
        )
    }

    #[test]
    fn refuses_empty_blueprint() {
        let err = SyntheticDatasetBlueprint::new(DatasetId::parse("dataset:1").unwrap(), vec![])
            .unwrap_err();
        assert!(matches!(err, SyntheticDomainError::EmptyBlueprint));
    }

    #[test]
    fn refuses_duplicate_case_ids() {
        let err = SyntheticDatasetBlueprint::new(
            DatasetId::parse("dataset:1").unwrap(),
            vec![
                case("case:1", KmpMcpCapability::Inspect),
                case("case:1", KmpMcpCapability::Ask),
            ],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            SyntheticDomainError::DuplicateCase { ref case_id } if case_id == "case:1"
        ));
    }

    #[test]
    fn for_all_capabilities_produces_one_case_per_kernel_tool() {
        let blueprint = SyntheticDatasetBlueprint::for_all_capabilities(
            DatasetId::parse("dataset:full").unwrap(),
            PositiveCount::parse(2, "minimum").unwrap(),
        )
        .unwrap();
        assert_eq!(blueprint.cases().len(), KmpMcpCapability::ALL.len());
        for case in blueprint.cases() {
            assert_eq!(case.minimum_examples().as_usize(), 2);
        }
    }

    #[test]
    fn for_all_generation_targets_includes_stop_and_escalate() {
        let blueprint = SyntheticDatasetBlueprint::for_all_generation_targets(
            DatasetId::parse("dataset:targets").unwrap(),
            PositiveCount::parse(2, "minimum").unwrap(),
        )
        .unwrap();
        assert_eq!(
            blueprint.cases().len(),
            SyntheticGenerationTarget::ALL.len()
        );
        assert!(
            blueprint
                .cases()
                .iter()
                .any(|case| case.target() == SyntheticGenerationTarget::Stop)
        );
        assert!(
            blueprint
                .cases()
                .iter()
                .any(|case| case.target() == SyntheticGenerationTarget::Escalate)
        );
    }
}
