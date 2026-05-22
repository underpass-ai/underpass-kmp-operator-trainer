//! Error returned by the realistic corpus production use case.

use thiserror::Error;

use crate::error::scenario_source_error::ScenarioSourceError;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BuildRealisticCorpusError {
    #[error(transparent)]
    Source(#[from] ScenarioSourceError),

    #[error("invalid realistic corpus configuration: {message}")]
    InvalidConfiguration { message: String },
}
