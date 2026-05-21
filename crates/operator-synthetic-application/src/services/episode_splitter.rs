//! Service that applies an episode split policy to episode specs.

use operator_shared_domain::error::domain_error::DomainError;
use operator_synthetic_domain::episode::bounded_ratio::BoundedRatio;
use operator_synthetic_domain::episode::episode_split_policy::EpisodeSplitPolicy;
use operator_synthetic_domain::episode::synthetic_episode_spec::SyntheticEpisodeSpec;
use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;

use crate::error::episode_split_error::EpisodeSplitError;
use crate::services::episode_split::EpisodeSplit;

#[derive(Debug, Default)]
pub struct EpisodeSplitter;

impl EpisodeSplitter {
    pub fn split(
        episodes: &[SyntheticEpisodeSpec],
        policy: &EpisodeSplitPolicy,
    ) -> Result<EpisodeSplit, EpisodeSplitError> {
        match policy {
            EpisodeSplitPolicy::ByAbout { eval_about_count } => {
                split_tail(episodes, eval_about_count.as_usize())
            }
            EpisodeSplitPolicy::ByEpisodeId { eval_episode_count } => {
                split_tail(episodes, eval_episode_count.as_usize())
            }
            EpisodeSplitPolicy::Stratified { eval_ratio, .. } => split_tail(
                episodes,
                stratified_eval_count(episodes.len(), *eval_ratio)?,
            ),
        }
    }
}

fn stratified_eval_count(
    total_episodes: usize,
    eval_ratio: BoundedRatio,
) -> Result<usize, EpisodeSplitError> {
    let desired_eval =
        usize_to_f64(total_episodes, "episode_split.total_episodes")? * eval_ratio.as_f64();
    let mut eval_count = 1;
    while usize_to_f64(eval_count, "episode_split.eval_count")? < desired_eval {
        eval_count += 1;
    }
    Ok(eval_count)
}

fn usize_to_f64(value: usize, context: &'static str) -> Result<f64, EpisodeSplitError> {
    let narrowed = u32::try_from(value).map_err(|_| {
        SyntheticDomainError::Shared(DomainError::UnsupportedValue {
            context,
            value: value.to_string(),
        })
    })?;
    Ok(f64::from(narrowed))
}

fn split_tail(
    episodes: &[SyntheticEpisodeSpec],
    eval_count: usize,
) -> Result<EpisodeSplit, EpisodeSplitError> {
    if eval_count >= episodes.len() {
        return Err(SyntheticDomainError::EvalSplitTooLarge {
            requested: eval_count,
            available: episodes.len(),
        }
        .into());
    }
    let split_at = episodes.len() - eval_count;
    Ok(EpisodeSplit::new(
        episodes[..split_at].to_vec(),
        episodes[split_at..].to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::episode::bounded_ratio::BoundedRatio;
    use operator_synthetic_domain::episode::capability_target::CapabilityTarget;
    use operator_synthetic_domain::episode::episode_id::EpisodeId;
    use operator_synthetic_domain::episode::episode_objective::EpisodeObjective;
    use operator_synthetic_domain::episode::episode_step_plan::EpisodeStepPlan;
    use operator_synthetic_domain::episode::episode_theme::EpisodeTheme;
    use operator_synthetic_domain::episode::stratum_key::StratumKey;

    fn episodes() -> Vec<SyntheticEpisodeSpec> {
        vec![
            episode("episode:1"),
            episode("episode:2"),
            episode("episode:3"),
        ]
    }

    fn episode(id: &str) -> SyntheticEpisodeSpec {
        SyntheticEpisodeSpec::new(
            EpisodeId::parse(id).unwrap(),
            EpisodeTheme::Incident,
            EpisodeObjective::parse("Resolve incident.").unwrap(),
            vec![EpisodeStepPlan::new(
                CapabilityTarget::new(
                    KmpMcpCapability::Inspect,
                    PositiveCount::parse(1, "minimum").unwrap(),
                ),
                EpisodeObjective::parse("Inspect evidence.").unwrap(),
            )],
        )
        .unwrap()
    }

    #[test]
    fn splits_by_episode_count() {
        let split = EpisodeSplitter::split(
            &episodes(),
            &EpisodeSplitPolicy::ByEpisodeId {
                eval_episode_count: PositiveCount::parse(1, "eval").unwrap(),
            },
        )
        .unwrap();
        assert_eq!(split.train().len(), 2);
        assert_eq!(split.eval().len(), 1);
    }

    #[test]
    fn splits_by_about_count_using_same_episode_boundary() {
        let split = EpisodeSplitter::split(
            &episodes(),
            &EpisodeSplitPolicy::ByAbout {
                eval_about_count: PositiveCount::parse(1, "eval").unwrap(),
            },
        )
        .unwrap();
        assert_eq!(split.train().len(), 2);
        assert_eq!(split.eval().len(), 1);
    }

    #[test]
    fn splits_by_stratified_ratio() {
        let split = EpisodeSplitter::split(
            &episodes(),
            &EpisodeSplitPolicy::Stratified {
                eval_ratio: BoundedRatio::parse(0.34).unwrap(),
                stratum_key: StratumKey::parse("theme").unwrap(),
            },
        )
        .unwrap();
        assert_eq!(split.train().len(), 1);
        assert_eq!(split.eval().len(), 2);
    }

    #[test]
    fn rejects_eval_count_that_consumes_every_episode() {
        let err = EpisodeSplitter::split(
            &episodes(),
            &EpisodeSplitPolicy::ByEpisodeId {
                eval_episode_count: PositiveCount::parse(3, "eval").unwrap(),
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            EpisodeSplitError::Domain(SyntheticDomainError::EvalSplitTooLarge { .. })
        ));
    }
}
