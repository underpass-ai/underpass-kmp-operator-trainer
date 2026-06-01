use serde::{Deserialize, Serialize};

use crate::budget_snapshot_dto::BudgetSnapshotDto;
use crate::coverage_deviation_snapshot_dto::CoverageDeviationSnapshotDto;
use crate::cursor_dto::CursorDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleStateDto {
    #[serde(default)]
    pub known_refs: Vec<String>,
    #[serde(default)]
    pub known_dimensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_cursor: Option<CursorDto>,
    pub budget: BudgetSnapshotDto,
    /// Online context-coverage deviation the operator perceives. Absent on
    /// corpora predating the signal (treated as "unknown" / maximal deviation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_deviation: Option<CoverageDeviationSnapshotDto>,
    /// Candidate sessions (abouts) the operator should cover for a cross-about
    /// goal — what retrieval surfaced. Empty/absent for single-about sessions
    /// (skip-if-empty keeps single-about wire snapshots unchanged).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_abouts: Vec<String>,
}
