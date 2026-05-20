//! `KmpMcpCapability` is the closed set of `KMP`/`MCP` capabilities the
//! synthetic context must cover. Each variant pairs a `KernelTool` with
//! its canonical `OperatorMode` for dataset planning purposes (a Wake
//! capability is planned in Read mode, a `WriteMemory` capability in
//! Write mode, etc.). Coverage tests in higher layers assert that every
//! variant has at least one case in any production blueprint.

use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KmpMcpCapability {
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

impl KmpMcpCapability {
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

    pub fn tool(self) -> KernelTool {
        match self {
            Self::Ingest => KernelTool::Ingest,
            Self::Wake => KernelTool::Wake,
            Self::Ask => KernelTool::Ask,
            Self::Near => KernelTool::Near,
            Self::Goto => KernelTool::Goto,
            Self::Rewind => KernelTool::Rewind,
            Self::Forward => KernelTool::Forward,
            Self::Trace => KernelTool::Trace,
            Self::Inspect => KernelTool::Inspect,
            Self::WriteMemory => KernelTool::WriteMemory,
        }
    }

    pub fn mode(self) -> OperatorMode {
        match self {
            Self::Ingest | Self::WriteMemory => OperatorMode::Write,
            _ => OperatorMode::Read,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Wake => "wake",
            Self::Ask => "ask",
            Self::Near => "near",
            Self::Goto => "goto",
            Self::Rewind => "rewind",
            Self::Forward => "forward",
            Self::Trace => "trace",
            Self::Inspect => "inspect",
            Self::WriteMemory => "write_memory",
        }
    }
}

impl std::fmt::Display for KmpMcpCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_capability_resolves_to_its_kernel_tool() {
        for capability in KmpMcpCapability::ALL {
            assert_eq!(
                capability.tool().as_str(),
                format!("kernel_{}", capability.name())
            );
        }
    }

    #[test]
    fn read_tools_use_read_mode() {
        for capability in KmpMcpCapability::ALL {
            let expected = if matches!(
                capability.tool(),
                KernelTool::Ingest | KernelTool::WriteMemory
            ) {
                OperatorMode::Write
            } else {
                OperatorMode::Read
            };
            assert_eq!(capability.mode(), expected);
        }
    }

    #[test]
    fn all_enumerates_every_variant_once() {
        let mut tools: Vec<KernelTool> = KmpMcpCapability::ALL.iter().map(|c| c.tool()).collect();
        tools.sort();
        let mut expected: Vec<KernelTool> = KernelTool::ALL.to_vec();
        expected.sort();
        assert_eq!(tools, expected);
    }
}
