use serde::{Deserialize, Serialize};

/// Wire form of the operator's context-coverage deviation snapshot. Carried on
/// `VisibleStateDto` (optional, so corpora predating the signal still parse) so
/// the policy/teacher perceives the online estimate. `deviation_milli` is
/// per-mille of `[0.0, 1.0]` (0 = fully covered, 1000 = nothing covered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageDeviationSnapshotDto {
    pub deviation_milli: u16,
    #[serde(default)]
    pub saturated: bool,
    #[serde(default)]
    pub conflict_blocking: bool,
}
