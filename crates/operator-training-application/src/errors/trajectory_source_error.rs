//! Error returned by the `TrajectorySource` port. Adapter-agnostic
//! shape so that filesystem, network, in-memory and stream adapters
//! all map onto the same variants.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrajectorySourceError {
    /// The adapter could not load any trajectories — file missing,
    /// connection refused, parsing failed at the boundary, etc.
    #[error("trajectory source '{adapter}' unavailable: {message}")]
    Unavailable {
        adapter: &'static str,
        message: String,
    },

    /// The adapter loaded data but a trajectory failed domain
    /// validation. The position is the index in the loaded stream.
    #[error("trajectory source '{adapter}' yielded invalid trajectory at index {index}: {message}")]
    InvalidTrajectory {
        adapter: &'static str,
        index: usize,
        message: String,
    },
}
