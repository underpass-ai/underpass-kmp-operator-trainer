use serde::{Deserialize, Serialize};

use crate::budget_snapshot_dto::BudgetSnapshotDto;
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
}
