//! JSON-RPC 2.0 error object. Carried inside `ToolsCallResponse.error`
//! when the server reports a protocol-level failure.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}
