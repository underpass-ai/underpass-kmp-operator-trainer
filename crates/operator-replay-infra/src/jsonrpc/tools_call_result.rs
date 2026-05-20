use serde::Deserialize;
use serde_json::Value;

use crate::jsonrpc::tools_call_result_content::ToolsCallResultContent;

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
