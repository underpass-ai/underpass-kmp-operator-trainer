//! Wire form of an authored window-expansion episode (one JSONL row).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowExpansionEpisodeDto {
    pub about: String,
    pub goal: String,
    pub initial_window: usize,
    pub max_iterations: usize,
    #[serde(default = "default_token_budget")]
    pub token_budget: u32,
    /// Gold set of period entry refs the window must cover. Absent (empty) means
    /// coverage is judged from the kernel's in-band signals instead.
    #[serde(default)]
    pub expected_refs: Vec<String>,
}

fn default_token_budget() -> u32 {
    8192
}
