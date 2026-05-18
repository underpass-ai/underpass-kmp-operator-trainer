use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetSnapshotDto {
    /// `None` represents the explicit "unbounded" sentinel; an absent
    /// field means the same thing on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls_remaining: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_remaining: Option<usize>,
}
