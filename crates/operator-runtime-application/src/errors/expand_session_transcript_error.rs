use thiserror::Error;

/// Why a [`crate::use_cases::expand_session_transcript_use_case`] expansion
/// failed. Both variants are defensive: a well-formed transcript produced by
/// the multi-step loop should never trigger them, because the loop already
/// validated every recorded action against the contract.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExpandSessionTranscriptError {
    #[error("failed to build trajectory identifier: {message}")]
    Id { message: String },

    #[error("transcript step is not a valid training trajectory: {message}")]
    Trajectory { message: String },
}
