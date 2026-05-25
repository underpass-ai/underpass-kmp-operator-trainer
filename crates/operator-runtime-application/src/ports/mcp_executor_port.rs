use operator_runtime_domain::session::observation::Observation;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::about_id::AboutId;

use crate::errors::mcp_executor_error::McpExecutorError;

pub trait McpExecutor: std::fmt::Debug + Send + Sync {
    fn execute(
        &self,
        action: &OperatorAction,
        about: &AboutId,
    ) -> Result<Observation, McpExecutorError>;
}
