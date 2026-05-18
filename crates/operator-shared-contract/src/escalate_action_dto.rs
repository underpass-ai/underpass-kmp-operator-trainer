use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalateActionDto {
    pub reason: String,
    pub target_model: String,
}
