//! Trait for validating a corpus snapshot against quality specifications.

use crate::quality::corpus_quality_violations::CorpusQualityViolations;
use crate::quality::corpus_snapshot::CorpusSnapshot;

pub trait CorpusQualityValidator: std::fmt::Debug + Send + Sync {
    fn validate(&self, snapshot: &CorpusSnapshot) -> Result<(), CorpusQualityViolations>;
}
