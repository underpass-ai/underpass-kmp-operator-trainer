//! Port for a frontier-LLM baseliner: an adapter that sends a chat
//! completion request and returns the raw assistant content.
//!
//! Parsing the response into an `OperatorAction` lives outside this
//! port. The baseliner only owns the LLM round-trip, so adapters can
//! target vLLM, `OpenAI`, Anthropic-via-OpenAI-compat, or a stub for
//! tests, all behind the same trait.

use crate::error::llm_baseliner_error::LlmBaselinerError;
use crate::ports::chat_message::ChatMessage;

pub trait LlmBaseliner: std::fmt::Debug + Send + Sync {
    /// Send `messages` to the LLM and return the assistant's content
    /// verbatim. The adapter is responsible for retries, timeouts,
    /// and protocol-level validation, but not for parsing the
    /// content into a domain action.
    fn complete(&self, messages: &[ChatMessage]) -> Result<String, LlmBaselinerError>;
}
