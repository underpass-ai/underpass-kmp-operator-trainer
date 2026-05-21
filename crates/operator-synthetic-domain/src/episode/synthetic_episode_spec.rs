//! Aggregate root for one realistic synthetic episode.

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_id::EpisodeId;
use crate::episode::episode_objective::EpisodeObjective;
use crate::episode::episode_step_plan::EpisodeStepPlan;
use crate::episode::episode_theme::EpisodeTheme;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticEpisodeSpec {
    episode_id: EpisodeId,
    theme: EpisodeTheme,
    objective: EpisodeObjective,
    steps: Vec<EpisodeStepPlan>,
}

impl SyntheticEpisodeSpec {
    pub fn new(
        episode_id: EpisodeId,
        theme: EpisodeTheme,
        objective: EpisodeObjective,
        steps: Vec<EpisodeStepPlan>,
    ) -> SyntheticDomainResult<Self> {
        if steps.is_empty() {
            return Err(SyntheticDomainError::EmptyEpisodeSteps);
        }
        Ok(Self {
            episode_id,
            theme,
            objective,
            steps,
        })
    }

    pub fn episode_id(&self) -> &EpisodeId {
        &self.episode_id
    }

    pub fn theme(&self) -> EpisodeTheme {
        self.theme
    }

    pub fn objective(&self) -> &EpisodeObjective {
        &self.objective
    }

    pub fn steps(&self) -> &[EpisodeStepPlan] {
        &self.steps
    }

    pub fn covers_capability(&self, capability: KmpMcpCapability) -> bool {
        self.steps()
            .iter()
            .any(|step| step.target().capability() == capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::capability_target::CapabilityTarget;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    fn step(capability: KmpMcpCapability) -> EpisodeStepPlan {
        EpisodeStepPlan::new(
            CapabilityTarget::new(capability, PositiveCount::parse(1, "minimum").unwrap()),
            EpisodeObjective::parse("Use the requested KMP capability.").unwrap(),
        )
    }

    #[test]
    fn builds_episode_with_steps() {
        let spec = SyntheticEpisodeSpec::new(
            EpisodeId::parse("episode:incident:1").unwrap(),
            EpisodeTheme::Incident,
            EpisodeObjective::parse("Resolve the incident.").unwrap(),
            vec![step(KmpMcpCapability::Near)],
        )
        .unwrap();
        assert_eq!(spec.episode_id().as_str(), "episode:incident:1");
        assert_eq!(spec.theme(), EpisodeTheme::Incident);
        assert_eq!(spec.objective().as_str(), "Resolve the incident.");
        assert!(spec.covers_capability(KmpMcpCapability::Near));
        assert_eq!(spec.clone(), spec);
    }

    #[test]
    fn rejects_episode_without_steps() {
        let err = SyntheticEpisodeSpec::new(
            EpisodeId::parse("episode:empty").unwrap(),
            EpisodeTheme::MemoryTask,
            EpisodeObjective::parse("Do nothing.").unwrap(),
            vec![],
        )
        .unwrap_err();
        assert!(matches!(err, SyntheticDomainError::EmptyEpisodeSteps));
    }
}
