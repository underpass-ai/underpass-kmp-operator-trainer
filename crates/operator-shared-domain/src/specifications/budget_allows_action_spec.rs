//! Specification: a `ToolCall` action consumes a call from the budget.
//! `Stop` and `Escalate` do not. If the action consumes a call and the
//! budget snapshot reports zero calls remaining, the action is rejected.

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::specifications::specification::Specification;

#[derive(Debug, Default)]
pub struct BudgetAllowsActionSpec;

impl BudgetAllowsActionSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for BudgetAllowsActionSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(_) = subject.action() else {
            return Ok(());
        };
        if subject.visible().budget().allows_another_call() {
            return Ok(());
        }
        Err(ContractViolation::new(
            ContractViolationCode::BudgetExhausted,
            "visible_state.budget.calls_remaining",
            "no remaining tool calls; action must be stop or escalate",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::stop_action::StopAction;
    use crate::action::stop_reason::StopReason;
    use crate::action::tool_call_action::ToolCallAction;
    use crate::mode::operator_mode::OperatorMode;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::tool_arguments::ToolArguments;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::visible_state::budget_snapshot::BudgetSnapshot;
    use crate::visible_state::visible_state_builder::VisibleStateBuilder;

    fn inspect_action() -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )))
    }

    #[test]
    fn accepts_when_budget_allows_a_call() {
        let visible = VisibleStateBuilder::new()
            .with_budget(BudgetSnapshot::bounded(1, 100))
            .build();
        let action = inspect_action();
        let subject = ActionContractSubject::new(&action, OperatorMode::Read, &visible);
        assert!(BudgetAllowsActionSpec::new().evaluate(&subject).is_ok());
    }

    #[test]
    fn rejects_when_no_calls_remain() {
        let visible = VisibleStateBuilder::new()
            .with_budget(BudgetSnapshot::bounded(0, 100))
            .build();
        let action = inspect_action();
        let subject = ActionContractSubject::new(&action, OperatorMode::Read, &visible);
        let err = BudgetAllowsActionSpec::new()
            .evaluate(&subject)
            .unwrap_err();
        assert_eq!(err.code(), ContractViolationCode::BudgetExhausted);
    }

    #[test]
    fn stop_action_is_allowed_even_when_budget_is_exhausted() {
        let visible = VisibleStateBuilder::new()
            .with_budget(BudgetSnapshot::bounded(0, 0))
            .build();
        let stop = OperatorAction::Stop(
            StopAction::new(StopReason::BudgetExhausted, None, vec![]).unwrap(),
        );
        let subject = ActionContractSubject::new(&stop, OperatorMode::Read, &visible);
        assert!(BudgetAllowsActionSpec::new().evaluate(&subject).is_ok());
    }
}
