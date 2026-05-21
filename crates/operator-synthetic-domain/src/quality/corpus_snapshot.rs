//! Subject evaluated by corpus-quality specifications.

use crate::dataset::synthetic_dataset::SyntheticDataset;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;
use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
use crate::quality::episode_split_snapshot::EpisodeSplitSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusSnapshot {
    dataset: SyntheticDataset,
    episodes: Vec<SyntheticEpisodeSpec>,
    audit: CorpusAuditSnapshot,
    episode_split: Option<EpisodeSplitSnapshot>,
}

impl CorpusSnapshot {
    pub fn new(
        dataset: SyntheticDataset,
        episodes: Vec<SyntheticEpisodeSpec>,
        audit: CorpusAuditSnapshot,
        episode_split: Option<EpisodeSplitSnapshot>,
    ) -> SyntheticDomainResult<Self> {
        if episodes.is_empty() {
            return Err(SyntheticDomainError::EmptyEpisodes);
        }
        Ok(Self {
            dataset,
            episodes,
            audit,
            episode_split,
        })
    }

    pub fn dataset(&self) -> &SyntheticDataset {
        &self.dataset
    }

    pub fn episodes(&self) -> &[SyntheticEpisodeSpec] {
        &self.episodes
    }

    pub fn audit(&self) -> &CorpusAuditSnapshot {
        &self.audit
    }

    pub fn episode_split(&self) -> Option<&EpisodeSplitSnapshot> {
        self.episode_split.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::test_support::{episode, inspect_dataset};

    #[test]
    fn builds_snapshot_with_episodes() {
        let snapshot = CorpusSnapshot::new(
            inspect_dataset(),
            vec![episode("episode:1")],
            CorpusAuditSnapshot::clean(),
            None,
        )
        .unwrap();
        assert_eq!(snapshot.episodes().len(), 1);
        assert_eq!(snapshot.clone(), snapshot);
    }

    #[test]
    fn rejects_empty_episodes() {
        let err = CorpusSnapshot::new(
            inspect_dataset(),
            vec![],
            CorpusAuditSnapshot::clean(),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, SyntheticDomainError::EmptyEpisodes));
    }
}
