//! Wire form of an authored cross-about count episode (one JSONL row).

use serde::{Deserialize, Serialize};

use crate::dto::cross_about_target_dto::CrossAboutTargetDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossAboutEpisodeDto {
    /// The abouts the session must visit, each with its gold operands. Must be
    /// non-empty; the first is the entry about.
    pub targets: Vec<CrossAboutTargetDto>,
    pub goal: String,
    pub initial_window: usize,
    pub max_iterations: usize,
    #[serde(default = "default_token_budget")]
    pub token_budget: u32,
}

fn default_token_budget() -> u32 {
    8192
}
