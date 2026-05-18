use operator_replay_domain::error::replay_domain_error::ReplayDomainError;
use thiserror::Error;

/// Error returned by `ExecuteReplayUseCase`. Individual tool-call
/// failures do **not** propagate here: they land in the resulting
/// `ReplayReport` as `ReplayOutcome::ToolCallFailed` entries. This
/// variant is reserved for failures that prevent the report from
/// being assembled at all (today: an empty predictions slice).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExecuteReplayError {
    #[error(transparent)]
    Domain(#[from] ReplayDomainError),
}
