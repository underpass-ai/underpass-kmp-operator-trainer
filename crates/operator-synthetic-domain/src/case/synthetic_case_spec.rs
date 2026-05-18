//! Specification of one synthetic case: the capability it covers plus the
//! minimum number of generated examples required.

use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticCaseSpec {
    case_id: SyntheticCaseId,
    capability: KmpMcpCapability,
    minimum_examples: PositiveCount,
}

impl SyntheticCaseSpec {
    pub fn new(
        case_id: SyntheticCaseId,
        capability: KmpMcpCapability,
        minimum_examples: PositiveCount,
    ) -> Self {
        Self {
            case_id,
            capability,
            minimum_examples,
        }
    }

    pub fn case_id(&self) -> &SyntheticCaseId {
        &self.case_id
    }

    pub fn capability(&self) -> KmpMcpCapability {
        self.capability
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
        assert_eq!(spec.capability(), KmpMcpCapability::Inspect);
        assert_eq!(spec.minimum_examples().as_usize(), 3);
    }
}
