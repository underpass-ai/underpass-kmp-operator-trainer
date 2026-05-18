use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteMemoryArgumentsDto {
    pub summary: String,
    pub body: String,
    #[serde(default)]
    pub related: Vec<String>,
}
