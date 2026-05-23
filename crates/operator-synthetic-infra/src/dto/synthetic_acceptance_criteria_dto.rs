use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticAcceptanceCriteriaDto {
    #[serde(default)]
    pub expected_stop_reason: Option<String>,
    #[serde(default)]
    pub expected_cursor_kind: Option<String>,
}
