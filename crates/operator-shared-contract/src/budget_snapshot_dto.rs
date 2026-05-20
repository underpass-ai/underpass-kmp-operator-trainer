use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetSnapshotDto {
    /// `None` represents the explicit "unbounded" sentinel; an absent
    /// field means the same thing on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls_remaining: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_remaining: Option<usize>,
}
