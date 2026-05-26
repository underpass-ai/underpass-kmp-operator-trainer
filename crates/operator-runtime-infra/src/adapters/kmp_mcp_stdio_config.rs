use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmpMcpStdioConfig {
    command: PathBuf,
    args: Vec<String>,
    grpc_endpoint: String,
    env: BTreeMap<String, String>,
}

impl KmpMcpStdioConfig {
    pub fn new(command: impl Into<PathBuf>, grpc_endpoint: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            grpc_endpoint: grpc_endpoint.into(),
            env: BTreeMap::new(),
        }
    }

    pub fn default_for_endpoint(grpc_endpoint: impl Into<String>) -> Self {
        Self::new("rehydration-mcp", grpc_endpoint)
    }

    #[must_use]
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn command(&self) -> &PathBuf {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn grpc_endpoint(&self) -> &str {
        &self.grpc_endpoint
    }

    pub fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }
}
