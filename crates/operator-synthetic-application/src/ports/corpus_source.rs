//! Port: load a typed corpus snapshot for quality evaluation.

use operator_synthetic_domain::quality::corpus_snapshot::CorpusSnapshot;

use crate::error::corpus_source_error::CorpusSourceError;

pub trait CorpusSource: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<CorpusSnapshot, CorpusSourceError>;
}
