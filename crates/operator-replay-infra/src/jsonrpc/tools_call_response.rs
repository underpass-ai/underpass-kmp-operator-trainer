//! Response envelope received from an MCP server for a `tools/call`
//! RPC. Either `result` is present and carries the tool's
//! structured content, or `error` is present and indicates a JSON-RPC
//! protocol-level failure.
//!
//! The MCP spec exposes the tool payload at either
//! `result.structuredContent` (modern) or
//! `result.content[0].text` as a JSON-encoded string (legacy). This
//! module accepts both shapes.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResponse {
    #[serde(default)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<ToolsCallResult>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResult {
    /// Newer MCP transport ships the typed payload here directly.
    #[serde(default, rename = "structuredContent")]
    pub structured_content: Option<Value>,
    /// Older MCP transport carries a `content` array; the first entry
    /// has `type: "text"` and a JSON-encoded string in `text`.
    #[serde(default)]
    pub content: Vec<ToolsCallResultContent>,
    /// Some MCP versions also expose `isError`; treat true as a tool-
    /// reported failure that the adapter surfaces as a protocol error.
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResultContent {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl ToolsCallResponse {
    /// Returns the structured content of a successful response, taking
    /// either the modern `structuredContent` field or the legacy
    /// `content[0].text` JSON-encoded payload. Returns the body
    /// description as an error string if the response is malformed.
    pub fn structured_content(&self) -> Result<Value, String> {
        let Some(result) = self.result.as_ref() else {
            return Err("response has no result".to_string());
        };
        if result.is_error {
            return Err("server reported is_error = true".to_string());
        }
        if let Some(value) = result.structured_content.as_ref() {
            return Ok(value.clone());
        }
        let first = result
            .content
            .first()
            .ok_or_else(|| "response result missing structuredContent and content".to_string())?;
        if first.kind != "text" {
            return Err(format!(
                "response content[0].type is '{}', expected 'text'",
                first.kind
            ));
        }
        serde_json::from_str(&first.text)
            .map_err(|e| format!("response content[0].text is not JSON: {e}"))
    }
}
