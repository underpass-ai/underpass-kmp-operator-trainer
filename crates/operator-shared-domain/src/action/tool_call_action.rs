use crate::tool::kernel_tool::KernelTool;
use crate::tool_arguments::tool_arguments::ToolArguments;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallAction {
    arguments: ToolArguments,
}

impl ToolCallAction {
    pub fn new(arguments: ToolArguments) -> Self {
        Self { arguments }
    }

    pub fn tool(&self) -> KernelTool {
        self.arguments.tool()
    }

    pub fn arguments(&self) -> &ToolArguments {
        &self.arguments
    }
}
