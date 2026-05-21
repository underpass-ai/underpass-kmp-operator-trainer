//! Error returned while loading a corpus for quality evaluation.

use thiserror::Error;

use crate::error::corpus_source_error::CorpusSourceError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluateCorpusQualityError {
    #[error(transparent)]
    Source(#[from] CorpusSourceError),
}
