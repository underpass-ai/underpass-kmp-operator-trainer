use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::ids::about_id::AboutId;
use crate::specifications::specification::Specification;
use crate::tool_arguments::tool_arguments::ToolArguments;

#[derive(Debug, Default)]
pub struct ActionAboutMatchesTrajectorySpec;

impl ActionAboutMatchesTrajectorySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for ActionAboutMatchesTrajectorySpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        match call.arguments() {
            ToolArguments::Ingest(args) => {
                check_about(subject.about(), args.about(), "ingest.about")
            }
            ToolArguments::Wake(args) => check_about(subject.about(), args.about(), "wake.about"),
            ToolArguments::Ask(_)
            | ToolArguments::Near(_)
            | ToolArguments::Goto(_)
            | ToolArguments::Rewind(_)
            | ToolArguments::Forward(_)
            | ToolArguments::Trace(_)
            | ToolArguments::Inspect(_)
            | ToolArguments::WriteMemory(_) => Ok(()),
        }
    }
}

fn check_about(
    trajectory_about: &AboutId,
    action_about: &AboutId,
    field: &str,
) -> Result<(), ContractViolation> {
    if trajectory_about == action_about {
        return Ok(());
    }
    Err(ContractViolation::new(
        ContractViolationCode::AboutMismatch,
        field.to_string(),
        format!(
            "action about '{action_about}' does not match trajectory about '{trajectory_about}'"
        ),
    ))
}
