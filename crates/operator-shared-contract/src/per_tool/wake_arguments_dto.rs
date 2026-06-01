use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WakeArgumentsDto {
    pub about: String,
    /// Opt-in cap on the entries wake surfaces (maps to the kernel's
    /// `budget.max_entries`). Absent = unbounded; skip-if-none keeps existing
    /// wake wire snapshots unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_entries: Option<u32>,
}
