//! `AllowedTools` is the closed set of `KernelTool` variants permitted by a
//! given `OperatorMode`. It is a value object: equality is by value, the
//! internal set is not exposed, and the only constructors are the named
//! factories `for_mode` (and `parse` for serialization round-trips).

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::mode::operator_mode::OperatorMode;
use crate::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedTools {
    mode: OperatorMode,
    tools: Vec<KernelTool>,
}

impl AllowedTools {
    pub fn for_mode(mode: OperatorMode) -> Self {
        let tools = match mode {
            OperatorMode::Read => vec![
                KernelTool::Wake,
                KernelTool::Ask,
                KernelTool::Near,
                KernelTool::Goto,
                KernelTool::Rewind,
                KernelTool::Forward,
                KernelTool::Trace,
                KernelTool::Inspect,
            ],
            OperatorMode::Write => vec![KernelTool::WriteMemory],
            OperatorMode::Full => vec![
                KernelTool::Wake,
                KernelTool::Ask,
                KernelTool::Near,
                KernelTool::Goto,
                KernelTool::Rewind,
                KernelTool::Forward,
                KernelTool::Trace,
                KernelTool::Inspect,
                KernelTool::WriteMemory,
            ],
            OperatorMode::WriterPreRead => vec![
                KernelTool::Wake,
                KernelTool::Ask,
                KernelTool::Near,
                KernelTool::Inspect,
            ],
        };
        Self { mode, tools }
    }

    /// Reconstruct from a mode and a candidate list. Refuses any tool that
    /// is not in `for_mode(mode)`'s list and refuses a list whose set
    /// differs from `for_mode(mode)`.
    pub fn parse(mode: OperatorMode, tools: Vec<KernelTool>) -> DomainResult<Self> {
        let canonical = Self::for_mode(mode);
        for tool in &tools {
            if !canonical.tools.contains(tool) {
                return Err(DomainError::ToolOutsideMode {
                    mode: mode.as_str(),
                    tool: tool.as_str(),
                });
            }
        }
        if tools.len() != canonical.tools.len() {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "allowed_tools.count",
                expected: canonical.tools.len().to_string(),
                actual: tools.len().to_string(),
            });
        }
        Ok(Self { mode, tools })
    }

    pub fn mode(&self) -> OperatorMode {
        self.mode
    }

    pub fn contains(&self, tool: KernelTool) -> bool {
        self.tools.contains(&tool)
    }

    pub fn as_slice(&self) -> &[KernelTool] {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_mode_excludes_write_memory() {
        let allowed = AllowedTools::for_mode(OperatorMode::Read);
        assert!(!allowed.contains(KernelTool::WriteMemory));
        assert!(allowed.contains(KernelTool::Inspect));
    }

    #[test]
    fn write_mode_only_allows_write_memory() {
        let allowed = AllowedTools::for_mode(OperatorMode::Write);
        assert_eq!(allowed.as_slice(), &[KernelTool::WriteMemory]);
    }

    #[test]
    fn full_mode_allows_every_tool() {
        let allowed = AllowedTools::for_mode(OperatorMode::Full);
        assert_eq!(allowed.as_slice().len(), KernelTool::ALL.len());
    }

    #[test]
    fn writer_pre_read_mode_is_restricted() {
        let allowed = AllowedTools::for_mode(OperatorMode::WriterPreRead);
        assert!(allowed.contains(KernelTool::Wake));
        assert!(!allowed.contains(KernelTool::WriteMemory));
        assert!(!allowed.contains(KernelTool::Trace));
    }

    #[test]
    fn parse_refuses_tool_outside_mode() {
        assert!(matches!(
            AllowedTools::parse(OperatorMode::Read, vec![KernelTool::WriteMemory]),
            Err(DomainError::ToolOutsideMode { .. })
        ));
    }
}
