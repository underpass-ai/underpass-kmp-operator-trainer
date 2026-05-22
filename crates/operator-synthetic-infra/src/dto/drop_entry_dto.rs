use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropEntryDto {
    pub scenario_id: String,
    pub target: String,
    pub reason: String,
    pub message: String,
}
