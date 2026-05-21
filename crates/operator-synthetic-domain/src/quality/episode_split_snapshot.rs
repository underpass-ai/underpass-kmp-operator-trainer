//! Domain snapshot of train/eval episode membership.

use std::collections::BTreeSet;

use crate::episode::episode_id::EpisodeId;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeSplitSnapshot {
    train: Vec<EpisodeId>,
    eval: Vec<EpisodeId>,
}

impl EpisodeSplitSnapshot {
    pub fn new(train: Vec<EpisodeId>, eval: Vec<EpisodeId>) -> SyntheticDomainResult<Self> {
        if train.is_empty() {
            return Err(SyntheticDomainError::EmptyTrainSplit);
        }
        if eval.is_empty() {
            return Err(SyntheticDomainError::EmptyEvalSplit);
        }
        ensure_unique(&train)?;
        ensure_unique(&eval)?;
        let train_ids: BTreeSet<&str> = train.iter().map(EpisodeId::as_str).collect();
        for episode_id in &eval {
            if train_ids.contains(episode_id.as_str()) {
                return Err(SyntheticDomainError::OverlappingEpisodeSplit {
                    episode_id: episode_id.as_str().to_string(),
                });
            }
        }
        Ok(Self { train, eval })
    }

    pub fn train(&self) -> &[EpisodeId] {
        &self.train
    }

    pub fn eval(&self) -> &[EpisodeId] {
        &self.eval
    }
}

fn ensure_unique(ids: &[EpisodeId]) -> SyntheticDomainResult<()> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for episode_id in ids {
        if !seen.insert(episode_id.as_str()) {
            return Err(SyntheticDomainError::DuplicateEpisodeInSplit {
                episode_id: episode_id.as_str().to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> EpisodeId {
        EpisodeId::parse(value).unwrap()
    }

    #[test]
    fn builds_non_overlapping_split() {
        let split = EpisodeSplitSnapshot::new(vec![id("episode:1")], vec![id("episode:2")])
            .expect("split is valid");
        assert_eq!(split.train()[0].as_str(), "episode:1");
        assert_eq!(split.eval()[0].as_str(), "episode:2");
        assert_eq!(split.clone(), split);
    }

    #[test]
    fn rejects_empty_or_overlapping_split() {
        assert!(matches!(
            EpisodeSplitSnapshot::new(vec![], vec![id("episode:2")]),
            Err(SyntheticDomainError::EmptyTrainSplit)
        ));
        assert!(matches!(
            EpisodeSplitSnapshot::new(vec![id("episode:1")], vec![id("episode:1")]),
            Err(SyntheticDomainError::OverlappingEpisodeSplit { .. })
        ));
    }
}
