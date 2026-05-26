use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteMemoryArgumentsDto {
    pub summary: String,
    pub body: String,
    #[serde(default)]
    pub related: Vec<String>,
}
