//! Specification: if the candidate action is a `ToolCall`, its tool must be
//! a member of `AllowedTools::for_mode(mode)`.

use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::mode::allowed_tools::AllowedTools;
use crate::specifications::specification::Specification;

#[derive(Debug, Default)]
pub struct ToolWithinModeSpec;

impl ToolWithinModeSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for ToolWithinModeSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let Some(tool) = subject.action().tool() else {
            return Ok(());
        };
        let allowed = AllowedTools::for_mode(subject.mode());
        if allowed.contains(tool) {
            return Ok(());
        }
        Err(ContractViolation::new(
            ContractViolationCode::ToolOutsideMode,
            "action.tool",
            format!(
                "tool {} is not allowed in mode {}",
                tool.as_str(),
                subject.mode().as_str()
            ),
        ))
    }
}
