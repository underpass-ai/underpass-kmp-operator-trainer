//! Typed prepared KMP/MCP action made visible to the teacher.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::tool::kernel_tool::KernelTool;

use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedOperatorAction {
    action: OperatorAction,
    tool: KernelTool,
}

impl PreparedOperatorAction {
    pub fn new(action: OperatorAction) -> SyntheticDomainResult<Self> {
        let tool =
            action
                .tool()
                .ok_or_else(|| SyntheticDomainError::PreparedActionMustBeToolCall {
                    kind: action.kind().as_str().to_string(),
                })?;
        Ok(Self { action, tool })
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }

    pub fn tool(&self) -> KernelTool {
        self.tool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;

    #[test]
    fn builds_for_tool_call_action() {
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:target").unwrap()),
        )));
        let prepared = PreparedOperatorAction::new(action.clone()).unwrap();
        assert_eq!(prepared.action(), &action);
        assert_eq!(prepared.tool(), KernelTool::Inspect);
    }

    #[test]
    fn rejects_stop_action() {
        let action =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        assert!(matches!(
            PreparedOperatorAction::new(action),
            Err(SyntheticDomainError::PreparedActionMustBeToolCall { .. })
        ));
    }
}
