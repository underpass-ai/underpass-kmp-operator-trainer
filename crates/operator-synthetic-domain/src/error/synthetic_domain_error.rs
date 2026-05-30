//! Domain error variants for the synthetic bounded context.

use operator_shared_domain::error::domain_error::DomainError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyntheticDomainError {
    #[error("synthetic blueprint must contain at least one case")]
    EmptyBlueprint,

    #[error("synthetic dataset must contain at least one trajectory")]
    EmptyDataset,

    #[error("synthetic episode must contain at least one step")]
    EmptyEpisodeSteps,

    #[error("corpus snapshot must contain at least one episode")]
    EmptyEpisodes,

    #[error("episode split must contain at least one train episode")]
    EmptyTrainSplit,

    #[error("episode split must contain at least one eval episode")]
    EmptyEvalSplit,

    #[error("episode split contains duplicate episode '{episode_id}'")]
    DuplicateEpisodeInSplit { episode_id: String },

    #[error("episode split has overlapping train/eval episode '{episode_id}'")]
    OverlappingEpisodeSplit { episode_id: String },

    #[error("requested eval episode count {requested} exceeds available episodes {available}")]
    EvalSplitTooLarge { requested: usize, available: usize },

    #[error("corpus quality validator must contain at least one specification")]
    EmptyCorpusQualityValidator,

    #[error("synthetic case '{case_id}' has duplicate occurrences in the blueprint")]
    DuplicateCase { case_id: String },

    #[error("calibration case must contain at least one accepted action")]
    EmptyAcceptedActions,

    #[error("calibration case accepted action count {actual} exceeds maximum {maximum}")]
    TooManyAcceptedActions { maximum: usize, actual: usize },

    #[error("calibration case accepted actions mix capabilities '{expected}' and '{actual}'")]
    MixedAcceptedActionCapability { expected: String, actual: String },

    #[error("prepared calibration action must be a tool call, got '{kind}'")]
    PreparedActionMustBeToolCall { kind: String },

    #[error("prepared calibration tool '{tool}' is not allowed in mode '{mode}'")]
    PreparedActionToolOutsideMode { mode: String, tool: String },

    #[error("cross-about episode must contain at least one target about")]
    EmptyCrossAboutTargets,

    #[error("cross-about target about '{about}' must carry at least one gold entry")]
    CrossAboutTargetMissingGold { about: String },

    #[error(transparent)]
    Shared(#[from] DomainError),
}
