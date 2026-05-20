//! `KernelTool` is the closed set of tools Operator may select.
//!
//! Adding a new variant is a deliberate domain change: every exhaustive
//! `match` over `KernelTool` in this crate breaks at compile time, by
//! design. Wildcard matches (`_ => ...`) on `KernelTool` are forbidden by
//! convention and reviewed for at PR time.

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelTool {
    Ingest,
    Wake,
    Ask,
    Near,
    Goto,
    Rewind,
    Forward,
    Trace,
    Inspect,
    WriteMemory,
}

impl KernelTool {
    /// Canonical name used in serialization and logs. Stable across versions.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "kernel_ingest",
            Self::Wake => "kernel_wake",
            Self::Ask => "kernel_ask",
            Self::Near => "kernel_near",
            Self::Goto => "kernel_goto",
            Self::Rewind => "kernel_rewind",
            Self::Forward => "kernel_forward",
            Self::Trace => "kernel_trace",
            Self::Inspect => "kernel_inspect",
            Self::WriteMemory => "kernel_write_memory",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "kernel_ingest" => Ok(Self::Ingest),
            "kernel_wake" => Ok(Self::Wake),
            "kernel_ask" => Ok(Self::Ask),
            "kernel_near" => Ok(Self::Near),
            "kernel_goto" => Ok(Self::Goto),
            "kernel_rewind" => Ok(Self::Rewind),
            "kernel_forward" => Ok(Self::Forward),
            "kernel_trace" => Ok(Self::Trace),
            "kernel_inspect" => Ok(Self::Inspect),
            "kernel_write_memory" => Ok(Self::WriteMemory),
            other => Err(DomainError::UnsupportedValue {
                context: "kernel_tool",
                value: other.to_string(),
            }),
        }
    }

    /// All variants in a stable order; used by `AllowedTools::for_mode` and
    /// architectural tests.
    pub const ALL: [Self; 10] = [
        Self::Ingest,
        Self::Wake,
        Self::Ask,
        Self::Near,
        Self::Goto,
        Self::Rewind,
        Self::Forward,
        Self::Trace,
        Self::Inspect,
        Self::WriteMemory,
    ];
}

impl std::fmt::Display for KernelTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_have_stable_names() {
        assert_eq!(KernelTool::Wake.as_str(), "kernel_wake");
        assert_eq!(KernelTool::Ingest.as_str(), "kernel_ingest");
        assert_eq!(KernelTool::Ask.as_str(), "kernel_ask");
        assert_eq!(KernelTool::Near.as_str(), "kernel_near");
        assert_eq!(KernelTool::Goto.as_str(), "kernel_goto");
        assert_eq!(KernelTool::Rewind.as_str(), "kernel_rewind");
        assert_eq!(KernelTool::Forward.as_str(), "kernel_forward");
        assert_eq!(KernelTool::Trace.as_str(), "kernel_trace");
        assert_eq!(KernelTool::Inspect.as_str(), "kernel_inspect");
        assert_eq!(KernelTool::WriteMemory.as_str(), "kernel_write_memory");
    }

    #[test]
    fn all_is_complete() {
        assert_eq!(KernelTool::ALL.len(), 10);
    }

    #[test]
    fn parse_round_trips_all_variants() {
        for tool in KernelTool::ALL {
            assert_eq!(KernelTool::parse(tool.as_str()).unwrap(), tool);
        }
    }

    #[test]
    fn parse_refuses_unknown_tool() {
        assert!(matches!(
            KernelTool::parse("kernel_dance"),
            Err(DomainError::UnsupportedValue {
                context: "kernel_tool",
                ..
            })
        ));
    }
}
