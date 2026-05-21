//! Plan for one operator decision inside a synthetic episode.

use crate::episode::capability_target::CapabilityTarget;
use crate::episode::episode_objective::EpisodeObjective;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeStepPlan {
    target: CapabilityTarget,
    objective: EpisodeObjective,
}

impl EpisodeStepPlan {
    pub fn new(target: CapabilityTarget, objective: EpisodeObjective) -> Self {
        Self { target, objective }
    }

    pub fn target(&self) -> &CapabilityTarget {
        &self.target
    }

    pub fn objective(&self) -> &EpisodeObjective {
        &self.objective
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    #[test]
    fn exposes_step_target_and_objective() {
        let step = EpisodeStepPlan::new(
            CapabilityTarget::new(
                KmpMcpCapability::Inspect,
                PositiveCount::parse(1, "minimum").unwrap(),
            ),
            EpisodeObjective::parse("Inspect the decisive evidence.").unwrap(),
        );
        assert_eq!(step.target().capability(), KmpMcpCapability::Inspect);
        assert_eq!(step.objective().as_str(), "Inspect the decisive evidence.");
        assert_eq!(step.clone(), step);
    }
}
