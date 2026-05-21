//! Required capability coverage target for an episode step.

use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityTarget {
    capability: KmpMcpCapability,
    minimum_examples: PositiveCount,
}

impl CapabilityTarget {
    pub fn new(capability: KmpMcpCapability, minimum_examples: PositiveCount) -> Self {
        Self {
            capability,
            minimum_examples,
        }
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
    fn exposes_target_parts() {
        let target = CapabilityTarget::new(
            KmpMcpCapability::Trace,
            PositiveCount::parse(2, "minimum").unwrap(),
        );
        assert_eq!(target.capability(), KmpMcpCapability::Trace);
        assert_eq!(target.minimum_examples().as_usize(), 2);
        assert_eq!(target.clone(), target);
    }
}
