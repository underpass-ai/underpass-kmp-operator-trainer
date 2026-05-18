//! Request envelope sent to an MCP server for a `tools/call` RPC.
//! JSON-RPC 2.0 shape. The `params` field carries the per-tool
//! arguments DTO under `arguments`.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ToolsCallRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    pub params: ToolsCallParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolsCallParams {
    pub name: String,
    pub arguments: Value,
}

impl ToolsCallRequest {
    pub fn new(id: u64, tool: impl Into<String>, arguments: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: "tools/call",
            params: ToolsCallParams {
                name: tool.into(),
                arguments,
            },
        }
    }
}
