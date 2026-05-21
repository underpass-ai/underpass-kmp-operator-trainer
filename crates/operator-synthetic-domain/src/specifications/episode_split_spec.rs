//! Corpus spec: train/eval split is episode-safe and references known episodes.

use std::collections::BTreeSet;

use operator_shared_domain::contract::contract_violation::ContractViolation;
use operator_shared_domain::contract::contract_violation_code::ContractViolationCode;
use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_snapshot::CorpusSnapshot;

#[derive(Debug, Default)]
pub struct EpisodeSplitSpec;

impl EpisodeSplitSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<CorpusSnapshot> for EpisodeSplitSpec {
    fn evaluate(&self, subject: &CorpusSnapshot) -> Result<(), ContractViolation> {
        let Some(split) = subject.episode_split() else {
            return Err(ContractViolation::new(
                ContractViolationCode::EpisodeSplit,
                "corpus.episode_split",
                "missing episode split",
            ));
        };
        let known: BTreeSet<&str> = subject
            .episodes()
            .iter()
            .map(|episode| episode.episode_id().as_str())
            .collect();
        for episode_id in split.train().iter().chain(split.eval().iter()) {
            if !known.contains(episode_id.as_str()) {
                return Err(ContractViolation::new(
                    ContractViolationCode::EpisodeSplit,
                    "corpus.episode_split",
                    format!("unknown split episode {}", episode_id.as_str()),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::test_support::{full_quality_snapshot, inspect_snapshot_without_split};

    #[test]
    fn accepts_known_episode_split() {
        EpisodeSplitSpec::new()
            .evaluate(&full_quality_snapshot())
            .unwrap();
    }

    #[test]
    fn rejects_missing_episode_split() {
        let err = EpisodeSplitSpec::new()
            .evaluate(&inspect_snapshot_without_split())
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::EpisodeSplit);
    }
}
