use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewindArgumentsDto {
    pub cursor_key: String,
    pub cursor_anchor: String,
    pub window: usize,
}
