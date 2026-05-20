use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ToolsCallParams {
    pub name: String,
    pub arguments: Value,
}
