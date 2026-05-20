//! One `(role, content)` pair as posted to an OpenAI-compatible
//! chat-completions endpoint. The request side mirrors but does not
//! reuse the response message DTO: requests carry `Serialize`,
//! responses carry `Deserialize`, and keeping them disjoint avoids
//! the per-direction `default`/`skip_serializing_if` attribute
//! collisions that would creep in if the two ever needed to diverge.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiChatCompletionRequestMessageDto {
    pub role: String,
    pub content: String,
}
