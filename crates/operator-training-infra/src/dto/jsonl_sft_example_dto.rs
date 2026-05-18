//! Wire shape of one JSONL SFT line per ADR 0012 §3:
//! `{"prompt": "<visible_state_text>", "completion": "<action_json>"}`.
//!
//! Both fields are strings. The dataset is one JSON object per line;
//! callers that need structured access deserialise per line.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct JsonlSftExampleDto {
    pub prompt: String,
    pub completion: String,
}
