//! `ToolOutcome` is the discriminated union over per-tool typed outcome
//! value objects. Mirrors the shape of `ToolArguments`: each variant
//! **is** a tool, and `ToolOutcome::tool()` returns the matching
//! `KernelTool` without ambiguity.

use crate::tool::kernel_tool::KernelTool;
use crate::tool_outcomes::ask_outcome::AskOutcome;
use crate::tool_outcomes::forward_outcome::ForwardOutcome;
use crate::tool_outcomes::goto_outcome::GotoOutcome;
use crate::tool_outcomes::inspect_outcome::InspectOutcome;
use crate::tool_outcomes::near_outcome::NearOutcome;
use crate::tool_outcomes::rewind_outcome::RewindOutcome;
use crate::tool_outcomes::trace_outcome::TraceOutcome;
use crate::tool_outcomes::wake_outcome::WakeOutcome;
use crate::tool_outcomes::write_memory_outcome::WriteMemoryOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOutcome {
    Wake(WakeOutcome),
    Ask(AskOutcome),
    Near(NearOutcome),
    Goto(GotoOutcome),
    Rewind(RewindOutcome),
    Forward(ForwardOutcome),
    Trace(TraceOutcome),
    Inspect(InspectOutcome),
    WriteMemory(WriteMemoryOutcome),
}

impl ToolOutcome {
    pub fn tool(&self) -> KernelTool {
        match self {
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
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::non_empty_string::NonEmptyString;

    #[test]
    fn variant_resolves_to_kernel_tool() {
        let outcome = ToolOutcome::Inspect(InspectOutcome::new(
            NonEmptyString::parse("s", "ctx").unwrap(),
            MemoryRef::parse("node:1").unwrap(),
            NonEmptyString::parse("claim", "ctx").unwrap(),
            vec![],
            vec![],
        ));
        assert_eq!(outcome.tool(), KernelTool::Inspect);
    }
}
