//! Error returned by the `LlmBaseliner` port. Adapter-agnostic shape
//! so HTTP-backed clients, future stdio clients, or test stubs all
//! map onto the same variants.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LlmBaselinerError {
    /// The adapter could not reach the LLM endpoint (DNS failure,
    /// TLS handshake broken, connection refused, request timed out).
    #[error("LLM baseliner '{adapter}' transport failure: {message}")]
    Transport {
        adapter: &'static str,
        message: String,
    },

    /// The LLM endpoint replied but the response shape did not match
    /// the chat-completions contract (missing `choices`, `message`,
    /// or `content`, non-JSON body, etc.).
    #[error("LLM baseliner '{adapter}' protocol failure: {message}")]
    Protocol {
        adapter: &'static str,
        message: String,
    },

    /// The endpoint accepted the request but returned an explicit
    /// API-level error payload (rate limit, billing, model not
    /// found, etc.). `code` is `None` when the server did not
    /// expose one.
    #[error("LLM baseliner '{adapter}' API error {code:?}: {message}")]
    ApiError {
        adapter: &'static str,
        code: Option<String>,
        message: String,
    },
}
