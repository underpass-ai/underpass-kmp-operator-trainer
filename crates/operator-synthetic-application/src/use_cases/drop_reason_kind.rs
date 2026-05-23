//! Stable bucket names for realistic corpus row drops.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DropReasonKind {
    TeacherError,
    ParseFailure,
    TargetMismatch,
    SemanticMismatch,
    ContractViolation,
    TrajectoryBuild,
}

impl DropReasonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TeacherError => "teacher_error",
            Self::ParseFailure => "parse_failure",
            Self::TargetMismatch => "target_mismatch",
            Self::SemanticMismatch => "semantic_mismatch",
            Self::ContractViolation => "contract_violation",
            Self::TrajectoryBuild => "trajectory_build",
        }
    }
}

impl std::fmt::Display for DropReasonKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
