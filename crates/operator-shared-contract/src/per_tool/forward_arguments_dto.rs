use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForwardArgumentsDto {
    pub cursor_key: String,
    pub cursor_anchor: String,
    pub window: usize,
}
