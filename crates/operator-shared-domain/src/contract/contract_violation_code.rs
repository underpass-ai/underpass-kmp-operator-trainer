#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractViolationCode {
    ToolOutsideMode,
    UnknownMemoryRef,
    UnknownDimension,
    CursorAnchorMissing,
    BudgetExhausted,
}

impl ContractViolationCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolOutsideMode => "tool_outside_mode",
            Self::UnknownMemoryRef => "unknown_memory_ref",
            Self::UnknownDimension => "unknown_dimension",
            Self::CursorAnchorMissing => "cursor_anchor_missing",
            Self::BudgetExhausted => "budget_exhausted",
        }
    }
}
