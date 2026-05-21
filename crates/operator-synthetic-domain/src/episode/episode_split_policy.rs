//! Closed strategy for splitting generated episodes into train/eval sets.

use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::episode::bounded_ratio::BoundedRatio;
use crate::episode::stratum_key::StratumKey;

#[derive(Debug, Clone, PartialEq)]
pub enum EpisodeSplitPolicy {
    ByAbout {
        eval_about_count: PositiveCount,
    },
    ByEpisodeId {
        eval_episode_count: PositiveCount,
    },
    Stratified {
        eval_ratio: BoundedRatio,
        stratum_key: StratumKey,
    },
}

impl EpisodeSplitPolicy {
    pub fn by_episode_id(eval_episode_count: PositiveCount) -> Self {
        Self::ByEpisodeId { eval_episode_count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_by_episode_id_policy() {
        let policy = EpisodeSplitPolicy::by_episode_id(
            PositiveCount::parse(2, "eval_episode_count").unwrap(),
        );
        assert_eq!(
            policy,
            EpisodeSplitPolicy::ByEpisodeId {
                eval_episode_count: PositiveCount::parse(2, "eval_episode_count").unwrap()
            }
        );
    }

    #[test]
    fn builds_stratified_policy_with_typed_values() {
        let policy = EpisodeSplitPolicy::Stratified {
            eval_ratio: BoundedRatio::parse(0.2).unwrap(),
            stratum_key: StratumKey::parse("theme").unwrap(),
        };
        assert!(matches!(policy, EpisodeSplitPolicy::Stratified { .. }));
    }
}
