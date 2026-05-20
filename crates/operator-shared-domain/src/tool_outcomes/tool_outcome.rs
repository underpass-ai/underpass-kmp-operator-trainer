//! `ToolOutcome` is the discriminated union over per-tool typed outcome
//! value objects. Mirrors the shape of `ToolArguments`: each variant
//! **is** a tool, and `ToolOutcome::tool()` returns the matching
//! `KernelTool` without ambiguity.

use crate::tool::kernel_tool::KernelTool;
use crate::tool_outcomes::ask_outcome::AskOutcome;
use crate::tool_outcomes::forward_outcome::ForwardOutcome;
use crate::tool_outcomes::goto_outcome::GotoOutcome;
use crate::tool_outcomes::ingest_outcome::IngestOutcome;
use crate::tool_outcomes::inspect_outcome::InspectOutcome;
use crate::tool_outcomes::near_outcome::NearOutcome;
use crate::tool_outcomes::rewind_outcome::RewindOutcome;
use crate::tool_outcomes::trace_outcome::TraceOutcome;
use crate::tool_outcomes::wake_outcome::WakeOutcome;
use crate::tool_outcomes::write_memory_outcome::WriteMemoryOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOutcome {
    Ingest(IngestOutcome),
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
    use crate::cursor::temporal_anchor::TemporalAnchor;
    use crate::cursor::temporal_cursor::TemporalCursor;
    use crate::cursor::temporal_cursor_key::TemporalCursorKey;
    use crate::ids::about_id::AboutId;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::non_empty_string::NonEmptyString;

    fn summary(text: &str) -> NonEmptyString {
        NonEmptyString::parse(text, "ctx").unwrap()
    }

    fn target() -> MemoryRef {
        MemoryRef::parse("node:1").unwrap()
    }

    #[test]
    fn every_variant_resolves_to_its_kernel_tool() {
        let cases: [(ToolOutcome, KernelTool); 10] = [
            (
                ToolOutcome::Ingest(IngestOutcome::new(
                    summary("ingest"),
                    AboutId::parse("about:1").unwrap(),
                    summary("memory:1"),
                    false,
                    vec![],
                )),
                KernelTool::Ingest,
            ),
            (
                ToolOutcome::Wake(WakeOutcome::new(summary("w"), vec![])),
                KernelTool::Wake,
            ),
            (
                ToolOutcome::Ask(AskOutcome::new(summary("a"), None, vec![])),
                KernelTool::Ask,
            ),
            (
                ToolOutcome::Near(NearOutcome::new(summary("n"), vec![])),
                KernelTool::Near,
            ),
            (
                ToolOutcome::Goto(GotoOutcome::new(summary("g"), vec![])),
                KernelTool::Goto,
            ),
            (
                ToolOutcome::Rewind(RewindOutcome::new(summary("r"), vec![])),
                KernelTool::Rewind,
            ),
            (
                ToolOutcome::Forward(ForwardOutcome::new(summary("f"), vec![])),
                KernelTool::Forward,
            ),
            (
                ToolOutcome::Trace(TraceOutcome::new(summary("t"), vec![])),
                KernelTool::Trace,
            ),
            (
                ToolOutcome::Inspect(InspectOutcome::new(
                    summary("i"),
                    target(),
                    summary("claim"),
                    vec![],
                    vec![],
                )),
                KernelTool::Inspect,
            ),
            (
                ToolOutcome::WriteMemory(WriteMemoryOutcome::new(
                    summary("wm"),
                    true,
                    false,
                    vec![],
                )),
                KernelTool::WriteMemory,
            ),
        ];
        // Touch every variant so a copy-paste bug in any match arm
        // surfaces here. The `TemporalCursor` import is kept so the
        // module compiles after future variants land that may need a
        // temporal-cursor fixture.
        let _temporal_marker = TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
        );
        for (outcome, expected) in cases {
            assert_eq!(outcome.tool(), expected);
        }
    }
}
