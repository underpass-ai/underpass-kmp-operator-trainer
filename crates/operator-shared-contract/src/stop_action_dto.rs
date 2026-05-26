use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopActionDto {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(default, alias = "final_refs", skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}
