//! Closed reason vocabulary for rows dropped during realistic corpus build.

use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_synthetic_domain::case::semantic_acceptance_violation::SemanticAcceptanceViolation;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::use_cases::drop_reason_kind::DropReasonKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropReason {
    TeacherError {
        message: String,
    },
    ParseFailure {
        message: String,
    },
    TargetMismatch {
        expected: SyntheticGenerationTarget,
        got_kind: String,
    },
    SemanticMismatch {
        violation: SemanticAcceptanceViolation,
    },
    ContractViolation {
        violations: ContractViolations,
    },
    TrajectoryBuild {
        message: String,
    },
}

impl DropReason {
    pub fn kind(&self) -> DropReasonKind {
        match self {
            Self::TeacherError { .. } => DropReasonKind::TeacherError,
            Self::ParseFailure { .. } => DropReasonKind::ParseFailure,
            Self::TargetMismatch { .. } => DropReasonKind::TargetMismatch,
            Self::SemanticMismatch { .. } => DropReasonKind::SemanticMismatch,
            Self::ContractViolation { .. } => DropReasonKind::ContractViolation,
            Self::TrajectoryBuild { .. } => DropReasonKind::TrajectoryBuild,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::TargetMismatch { expected, got_kind } => {
                format!("expected {}, got {got_kind}", expected.name())
            }
            Self::SemanticMismatch { violation } => violation.message(),
            Self::ContractViolation { violations } => format!("{violations:?}"),
            Self::TeacherError { message }
            | Self::ParseFailure { message }
            | Self::TrajectoryBuild { message } => message.clone(),
        }
    }
}
