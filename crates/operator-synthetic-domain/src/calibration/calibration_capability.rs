//! Calibration bucket used for per-capability teacher metrics.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CalibrationCapability {
    KernelIngest,
    KernelWake,
    KernelAsk,
    KernelNear,
    KernelGoto,
    KernelRewind,
    KernelForward,
    KernelTrace,
    KernelInspect,
    KernelWriteMemory,
    Stop,
    Escalate,
}

impl CalibrationCapability {
    pub const ALL: [Self; 12] = [
        Self::KernelIngest,
        Self::KernelWake,
        Self::KernelAsk,
        Self::KernelNear,
        Self::KernelGoto,
        Self::KernelRewind,
        Self::KernelForward,
        Self::KernelTrace,
        Self::KernelInspect,
        Self::KernelWriteMemory,
        Self::Stop,
        Self::Escalate,
    ];

    pub fn from_action(action: &OperatorAction) -> Self {
        match action {
            OperatorAction::ToolCall(call) => Self::from_tool(call.tool()),
            OperatorAction::Stop(_) => Self::Stop,
            OperatorAction::Escalate(_) => Self::Escalate,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::KernelIngest => "kernel_ingest",
            Self::KernelWake => "kernel_wake",
            Self::KernelAsk => "kernel_ask",
            Self::KernelNear => "kernel_near",
            Self::KernelGoto => "kernel_goto",
            Self::KernelRewind => "kernel_rewind",
            Self::KernelForward => "kernel_forward",
            Self::KernelTrace => "kernel_trace",
            Self::KernelInspect => "kernel_inspect",
            Self::KernelWriteMemory => "kernel_write_memory",
            Self::Stop => "stop",
            Self::Escalate => "escalate",
        }
    }

    fn from_tool(tool: KernelTool) -> Self {
        match tool {
            KernelTool::Ingest => Self::KernelIngest,
            KernelTool::Wake => Self::KernelWake,
            KernelTool::Ask => Self::KernelAsk,
            KernelTool::Near => Self::KernelNear,
            KernelTool::Goto => Self::KernelGoto,
            KernelTool::Rewind => Self::KernelRewind,
            KernelTool::Forward => Self::KernelForward,
            KernelTool::Trace => Self::KernelTrace,
            KernelTool::Inspect => Self::KernelInspect,
            KernelTool::WriteMemory => Self::KernelWriteMemory,
        }
    }
}

impl std::fmt::Display for CalibrationCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
    fn all_contains_twelve_buckets() {
        assert_eq!(CalibrationCapability::ALL.len(), 12);
    }

    #[test]
    fn maps_actions_to_metric_bucket() {
        let inspect = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )));
        assert_eq!(
            CalibrationCapability::from_action(&inspect),
            CalibrationCapability::KernelInspect
        );
        let stop =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        assert_eq!(
            CalibrationCapability::from_action(&stop),
            CalibrationCapability::Stop
        );
    }
}
