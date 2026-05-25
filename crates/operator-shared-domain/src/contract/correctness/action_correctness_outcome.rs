use crate::action::operator_action_kind::OperatorActionKind;
use crate::contract::correctness::correctness_mode::CorrectnessMode;
use crate::contract::correctness::field_outcome::FieldOutcome;
use crate::contract::correctness::field_path::FieldPath;
use crate::contract::correctness::field_result::FieldResult;
use crate::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCorrectnessOutcome {
    field_results: Vec<FieldResult>,
}

impl ActionCorrectnessOutcome {
    pub fn new(field_results: Vec<FieldResult>) -> Self {
        Self { field_results }
    }

    pub fn kind_mismatch(actual: OperatorActionKind, expected: OperatorActionKind) -> Self {
        Self::new(vec![FieldResult::new(
            FieldPath::trusted_static("kind"),
            CorrectnessMode::Exact,
            FieldOutcome::Fail {
                expected: expected.as_str().to_string(),
                actual: actual.as_str().to_string(),
            },
        )])
    }

    pub fn tool_mismatch(actual: KernelTool, expected: KernelTool) -> Self {
        Self::new(vec![FieldResult::new(
            FieldPath::trusted_static("tool"),
            CorrectnessMode::Exact,
            FieldOutcome::Fail {
                expected: expected.as_str().to_string(),
                actual: actual.as_str().to_string(),
            },
        )])
    }

    pub fn is_correct(&self) -> bool {
        self.field_results.iter().all(FieldResult::is_correct)
    }

    pub fn failed_fields(&self) -> impl Iterator<Item = &FieldResult> {
        self.field_results
            .iter()
            .filter(|result| !result.is_correct())
    }

    pub fn field_results(&self) -> &[FieldResult] {
        &self.field_results
    }
}
