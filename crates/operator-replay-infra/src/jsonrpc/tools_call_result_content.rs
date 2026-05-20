use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResultContent {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}
