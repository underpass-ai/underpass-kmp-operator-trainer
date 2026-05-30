use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NearArgumentsDto {
    pub anchor: String,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Temporal window around the anchor (entries before/after). Absent means
    /// anchor-only; window expansion grows these to cover a period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_entries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_entries: Option<u32>,
}
