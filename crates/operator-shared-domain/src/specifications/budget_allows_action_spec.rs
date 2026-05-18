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
