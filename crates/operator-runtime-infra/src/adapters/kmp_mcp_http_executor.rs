use operator_replay_application::error::kmp_client_error::KmpClientError;
use operator_replay_application::ports::kmp_mcp_client::KmpMcpClient;
use operator_replay_infra::adapters::http_kmp_mcp_client::HttpKmpMcpClient;
use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_domain::session::observation::Observation;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;

#[derive(Debug)]
pub struct KmpMcpHttpExecutor {
    client: HttpKmpMcpClient,
}

impl KmpMcpHttpExecutor {
    pub fn new(endpoint: impl Into<String>) -> Result<Self, McpExecutorError> {
        let client = HttpKmpMcpClient::new(endpoint).map_err(map_kmp_error)?;
        Ok(Self { client })
    }

    pub fn with_client(client: HttpKmpMcpClient) -> Self {
        Self { client }
    }
}

impl McpExecutor for KmpMcpHttpExecutor {
    fn execute(
        &self,
        action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        let OperatorAction::ToolCall(call) = action else {
            return Err(McpExecutorError::UnsupportedTool {
                tool: "terminal_action",
            });
        };

        let outcome = match call.arguments() {
            ToolArguments::Ingest(_) | ToolArguments::WriteMemory(_) => {
                return Err(McpExecutorError::WriteToolNotAllowedInReadProfile);
            }
            ToolArguments::Wake(args) => {
                ToolOutcome::Wake(self.client.wake(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Ask(args) => {
                ToolOutcome::Ask(self.client.ask(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Near(args) => {
                ToolOutcome::Near(self.client.near(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Goto(args) => {
                ToolOutcome::Goto(self.client.goto(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Rewind(args) => {
                ToolOutcome::Rewind(self.client.rewind(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Forward(args) => {
                ToolOutcome::Forward(self.client.forward(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Trace(args) => {
                ToolOutcome::Trace(self.client.trace(args).map_err(map_kmp_error)?)
            }
            ToolArguments::Inspect(args) => {
                ToolOutcome::Inspect(self.client.inspect(args).map_err(map_kmp_error)?)
            }
        };
        let observed_refs = observed_refs_for(&outcome);
        Ok(Observation::ToolResponse {
            outcome,
            observed_refs,
        })
    }
}

fn map_kmp_error(error: KmpClientError) -> McpExecutorError {
    match error {
        KmpClientError::Transport { message, .. } => McpExecutorError::Transport { message },
        KmpClientError::Protocol { message, .. }
        | KmpClientError::InvalidArguments { message, .. }
        | KmpClientError::MalformedResponse { message, .. } => {
            McpExecutorError::Protocol { message }
        }
    }
}

fn observed_refs_for(outcome: &ToolOutcome) -> Vec<MemoryRef> {
    match outcome {
        ToolOutcome::Ingest(_) | ToolOutcome::WriteMemory(_) => vec![],
        ToolOutcome::Wake(o) => o.surfaced_refs().to_vec(),
        ToolOutcome::Ask(o) => o.evidence_refs().to_vec(),
        ToolOutcome::Near(o) => o.entry_refs().to_vec(),
        ToolOutcome::Goto(o) => o.entry_refs().to_vec(),
        ToolOutcome::Rewind(o) => o.entry_refs().to_vec(),
        ToolOutcome::Forward(o) => o.entry_refs().to_vec(),
        ToolOutcome::Trace(o) => o.path_refs().to_vec(),
        ToolOutcome::Inspect(o) => {
            let mut refs =
                Vec::with_capacity(1 + o.incoming_refs().len() + o.outgoing_refs().len());
            refs.push(o.target().clone());
            refs.extend(o.incoming_refs().iter().cloned());
            refs.extend(o.outgoing_refs().iter().cloned());
            refs
        }
    }
}
