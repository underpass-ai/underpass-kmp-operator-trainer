//! `ToolArguments` is the discriminated union over per-tool typed argument
//! value objects. The variant **is** the tool; there is no way to build a
//! `ToolArguments::Wake(_)` and claim it is a `Near` call.

use crate::tool::kernel_tool::KernelTool;
use crate::tool_arguments::ask_arguments::AskArguments;
use crate::tool_arguments::forward_arguments::ForwardArguments;
use crate::tool_arguments::goto_arguments::GotoArguments;
use crate::tool_arguments::ingest_arguments::IngestArguments;
use crate::tool_arguments::inspect_arguments::InspectArguments;
use crate::tool_arguments::near_arguments::NearArguments;
use crate::tool_arguments::rewind_arguments::RewindArguments;
use crate::tool_arguments::trace_arguments::TraceArguments;
use crate::tool_arguments::wake_arguments::WakeArguments;
use crate::tool_arguments::write_memory_arguments::WriteMemoryArguments;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolArguments {
    Ingest(IngestArguments),
    Wake(WakeArguments),
    Ask(AskArguments),
    Near(NearArguments),
    Goto(GotoArguments),
    Rewind(RewindArguments),
    Forward(ForwardArguments),
    Trace(TraceArguments),
    Inspect(InspectArguments),
    WriteMemory(WriteMemoryArguments),
}

impl ToolArguments {
    pub fn tool(&self) -> KernelTool {
        match self {
            Self::Ingest(_) => KernelTool::Ingest,
            Self::Wake(_) => KernelTool::Wake,
            Self::Ask(_) => KernelTool::Ask,
            Self::Near(_) => KernelTool::Near,
            Self::Goto(_) => KernelTool::Goto,
            Self::Rewind(_) => KernelTool::Rewind,
            Self::Forward(_) => KernelTool::Forward,
            Self::Trace(_) => KernelTool::Trace,
            Self::Inspect(_) => KernelTool::Inspect,
            Self::WriteMemory(_) => KernelTool::WriteMemory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::about_id::AboutId;

    #[test]
    fn variant_matches_kernel_tool() {
        let args = ToolArguments::Wake(WakeArguments::new(AboutId::parse("about:1").unwrap()));
        assert_eq!(args.tool(), KernelTool::Wake);
    }
}
