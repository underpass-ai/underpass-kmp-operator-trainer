#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractViolationCode {
    AboutMismatch,
    ToolOutsideMode,
    UnknownMemoryRef,
    UnknownDimension,
    CursorAnchorMissing,
    BudgetExhausted,
    SchemaParse,
    ActionParse,
    ContractCoverage,
    ModeSafety,
    ReferenceSafety,
    ScopeSafety,
    PaginationSafety,
    WriteProof,
    NoGoldAudit,
    EpisodeSplit,
    DuplicateAudit,
    ReplaySmoke,
    FrontierCeiling,
}

impl ContractViolationCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AboutMismatch => "about_mismatch",
            Self::ToolOutsideMode => "tool_outside_mode",
            Self::UnknownMemoryRef => "unknown_memory_ref",
            Self::UnknownDimension => "unknown_dimension",
            Self::CursorAnchorMissing => "cursor_anchor_missing",
            Self::BudgetExhausted => "budget_exhausted",
            Self::SchemaParse => "schema_parse",
            Self::ActionParse => "action_parse",
            Self::ContractCoverage => "contract_coverage",
            Self::ModeSafety => "mode_safety",
            Self::ReferenceSafety => "reference_safety",
            Self::ScopeSafety => "scope_safety",
            Self::PaginationSafety => "pagination_safety",
            Self::WriteProof => "write_proof",
            Self::NoGoldAudit => "no_gold_audit",
            Self::EpisodeSplit => "episode_split",
            Self::DuplicateAudit => "duplicate_audit",
            Self::ReplaySmoke => "replay_smoke",
            Self::FrontierCeiling => "frontier_ceiling",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_quality_codes_have_stable_names() {
        assert_eq!(ContractViolationCode::SchemaParse.as_str(), "schema_parse");
        assert_eq!(ContractViolationCode::ActionParse.as_str(), "action_parse");
        assert_eq!(
            ContractViolationCode::ContractCoverage.as_str(),
            "contract_coverage"
        );
        assert_eq!(ContractViolationCode::ModeSafety.as_str(), "mode_safety");
        assert_eq!(
            ContractViolationCode::ReferenceSafety.as_str(),
            "reference_safety"
        );
        assert_eq!(ContractViolationCode::ScopeSafety.as_str(), "scope_safety");
        assert_eq!(
            ContractViolationCode::PaginationSafety.as_str(),
            "pagination_safety"
        );
        assert_eq!(ContractViolationCode::WriteProof.as_str(), "write_proof");
        assert_eq!(ContractViolationCode::NoGoldAudit.as_str(), "no_gold_audit");
        assert_eq!(
            ContractViolationCode::EpisodeSplit.as_str(),
            "episode_split"
        );
        assert_eq!(
            ContractViolationCode::DuplicateAudit.as_str(),
            "duplicate_audit"
        );
        assert_eq!(ContractViolationCode::ReplaySmoke.as_str(), "replay_smoke");
        assert_eq!(
            ContractViolationCode::FrontierCeiling.as_str(),
            "frontier_ceiling"
        );
    }
}
