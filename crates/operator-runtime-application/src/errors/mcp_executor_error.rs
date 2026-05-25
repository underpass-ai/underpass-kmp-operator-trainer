use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpExecutorError {
    #[error("write tool is not allowed in read-profile runtime")]
    WriteToolNotAllowedInReadProfile,

    #[error("MCP transport error: {message}")]
    Transport { message: String },

    #[error("MCP protocol error: {message}")]
    Protocol { message: String },

    #[error("MCP tool is not supported by this runtime adapter: {tool}")]
    UnsupportedTool { tool: &'static str },
}
