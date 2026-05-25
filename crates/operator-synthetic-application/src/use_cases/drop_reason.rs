//! Closed reason vocabulary for rows dropped during realistic corpus build.

use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_synthetic_domain::case::semantic_acceptance_violation::SemanticAcceptanceViolation;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::use_cases::drop_reason_kind::DropReasonKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropReason {
    TeacherError {
        message: String,
    },
    TeacherTruncation {
        finish_reason: FinishReason,
        content_len: usize,
        raw_content_tail: String,
        request_bytes: usize,
        response_bytes: usize,
        elapsed_ms: u128,
        request_id: Option<String>,
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
            Self::TeacherTruncation { .. } => DropReasonKind::TeacherTruncation,
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
            Self::TeacherTruncation {
                finish_reason,
                content_len,
                raw_content_tail,
                request_bytes,
                response_bytes,
                elapsed_ms,
                request_id,
            } => format!(
                "teacher finished with {} before producing a parseable action; content_len={content_len}; raw_content_tail={raw_content_tail}; request_bytes={request_bytes}; response_bytes={response_bytes}; elapsed_ms={elapsed_ms}; request_id={request_id:?}",
                finish_reason.as_str(),
            ),
            Self::TeacherError { message }
            | Self::ParseFailure { message }
            | Self::TrajectoryBuild { message } => message.clone(),
        }
    }

    pub fn raw_content_tail(&self) -> Option<&str> {
        match self {
            Self::TeacherTruncation {
                raw_content_tail, ..
            } => Some(raw_content_tail),
            _ => None,
        }
    }
}
