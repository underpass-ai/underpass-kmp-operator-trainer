//! Specification of one synthetic case: the generation target it covers plus
//! the minimum number of generated examples required.

use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::case::synthetic_generation_target::SyntheticGenerationTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticCaseSpec {
    case_id: SyntheticCaseId,
    target: SyntheticGenerationTarget,
    minimum_examples: PositiveCount,
}

impl SyntheticCaseSpec {
    pub fn new(
        case_id: SyntheticCaseId,
        capability: KmpMcpCapability,
        minimum_examples: PositiveCount,
    ) -> Self {
        Self::for_target(
            case_id,
            SyntheticGenerationTarget::from(capability),
            minimum_examples,
        )
    }

    pub fn for_target(
        case_id: SyntheticCaseId,
        target: SyntheticGenerationTarget,
        minimum_examples: PositiveCount,
    ) -> Self {
        Self {
            case_id,
            target,
            minimum_examples,
        }
    }

    pub fn case_id(&self) -> &SyntheticCaseId {
        &self.case_id
    }

    pub fn target(&self) -> SyntheticGenerationTarget {
        self.target
    }

    pub fn kmp_capability(&self) -> Option<KmpMcpCapability> {
        self.target.kmp_capability()
    }

    pub fn minimum_examples(&self) -> PositiveCount {
        self.minimum_examples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_id_capability_and_minimum() {
        let spec = SyntheticCaseSpec::new(
            SyntheticCaseId::parse("case:1").unwrap(),
            KmpMcpCapability::Inspect,
            PositiveCount::parse(3, "minimum").unwrap(),
        );
        assert_eq!(spec.case_id().as_str(), "case:1");
        assert_eq!(
            spec.target(),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect)
        );
        assert_eq!(spec.kmp_capability(), Some(KmpMcpCapability::Inspect));
        assert_eq!(spec.minimum_examples().as_usize(), 3);
    }

    #[test]
    fn can_target_stop_or_escalate_without_a_kmp_capability() {
        let spec = SyntheticCaseSpec::for_target(
            SyntheticCaseId::parse("case:stop").unwrap(),
            SyntheticGenerationTarget::Stop,
            PositiveCount::parse(1, "minimum").unwrap(),
        );
        assert_eq!(spec.target(), SyntheticGenerationTarget::Stop);
        assert_eq!(spec.kmp_capability(), None);
    }
}
