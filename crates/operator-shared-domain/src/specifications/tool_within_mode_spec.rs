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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::operator_action::OperatorAction;
    use crate::action::stop_action::StopAction;
    use crate::action::stop_reason::StopReason;
    use crate::action::tool_call_action::ToolCallAction;
    use crate::ids::about_id::AboutId;
    use crate::mode::operator_mode::OperatorMode;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::tool_arguments::ToolArguments;
    use crate::tool_arguments::write_memory_arguments::WriteMemoryArguments;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::visible_state::visible_state_builder::VisibleStateBuilder;

    fn about() -> AboutId {
        AboutId::parse("about:tool").unwrap()
    }

    #[test]
    fn accepts_inspect_in_read_mode() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )));
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        assert!(ToolWithinModeSpec::new().evaluate(&subject).is_ok());
    }

    #[test]
    fn rejects_write_memory_in_read_mode() {
        let visible = VisibleStateBuilder::new().build();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
            WriteMemoryArguments::new("s", "b", vec![]).unwrap(),
        )));
        let case_about = about();
        let subject =
            ActionContractSubject::new(&action, &case_about, OperatorMode::Read, &visible);
        let err = ToolWithinModeSpec::new().evaluate(&subject).unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::ToolOutsideMode);
    }

    #[test]
    fn ignores_stop_and_escalate_actions() {
        let visible = VisibleStateBuilder::new().build();
        let stop =
            OperatorAction::Stop(StopAction::new(StopReason::NoCandidate, None, vec![]).unwrap());
        let case_about = about();
        let subject = ActionContractSubject::new(&stop, &case_about, OperatorMode::Read, &visible);
        assert!(ToolWithinModeSpec::new().evaluate(&subject).is_ok());
    }
}
