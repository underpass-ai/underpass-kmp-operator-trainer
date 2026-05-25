use std::time::Duration;

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::contract_violations::ContractViolations;

use crate::budget::session_budget::SessionBudget;
use crate::error::runtime_domain_result::RuntimeDomainResult;
use crate::session::observation::Observation;
use crate::session::operator_request::OperatorRequest;
use crate::session::operator_session_id::OperatorSessionId;
use crate::session::outcome_class::OutcomeClass;
use crate::session::terminal_reason::TerminalReason;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOutcome {
    session_id: OperatorSessionId,
    predicted_action: OperatorAction,
    observation: Option<Observation>,
    outcome_class: OutcomeClass,
    elapsed_ms: u64,
    final_budget: SessionBudget,
}

impl SessionOutcome {
    pub fn contract_violation(
        request: &OperatorRequest,
        predicted_action: OperatorAction,
        violations: ContractViolations,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            request,
            predicted_action,
            None,
            OutcomeClass::ContractViolation { violations },
            elapsed,
            request.initial_budget(),
        )
    }

    pub fn budget_exhausted(
        request: &OperatorRequest,
        predicted_action: OperatorAction,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            request,
            predicted_action,
            None,
            OutcomeClass::BudgetExhausted,
            elapsed,
            request.initial_budget(),
        )
    }

    pub fn terminal(
        request: &OperatorRequest,
        predicted_action: OperatorAction,
        reason: TerminalReason,
        elapsed: Duration,
    ) -> Self {
        let outcome_class = match reason {
            TerminalReason::Stop => OutcomeClass::Completed,
            TerminalReason::Escalate => OutcomeClass::Escalated,
        };
        Self::new(
            request,
            predicted_action,
            Some(Observation::Terminal { reason }),
            outcome_class,
            elapsed,
            request.initial_budget(),
        )
    }

    pub fn from_observation(
        request: &OperatorRequest,
        predicted_action: OperatorAction,
        observation: Observation,
        elapsed: Duration,
    ) -> RuntimeDomainResult<Self> {
        let final_budget = request.initial_budget().try_consume_call()?;
        let outcome_class = match &observation {
            Observation::ToolResponse { .. } => OutcomeClass::Completed,
            Observation::ToolError { code, .. } => {
                OutcomeClass::McpExecutionFailure { code: code.clone() }
            }
            Observation::Terminal { reason } => match reason {
                TerminalReason::Stop => OutcomeClass::Completed,
                TerminalReason::Escalate => OutcomeClass::Escalated,
            },
        };
        Ok(Self::new(
            request,
            predicted_action,
            Some(observation),
            outcome_class,
            elapsed,
            final_budget,
        ))
    }

    fn new(
        request: &OperatorRequest,
        predicted_action: OperatorAction,
        observation: Option<Observation>,
        outcome_class: OutcomeClass,
        elapsed: Duration,
        final_budget: SessionBudget,
    ) -> Self {
        Self {
            session_id: request.session_id().clone(),
            predicted_action,
            observation,
            outcome_class,
            elapsed_ms: elapsed.as_millis().try_into().unwrap_or(u64::MAX),
            final_budget,
        }
    }

    pub fn session_id(&self) -> &OperatorSessionId {
        &self.session_id
    }

    pub fn predicted_action(&self) -> &OperatorAction {
        &self.predicted_action
    }

    pub fn observation(&self) -> Option<&Observation> {
        self.observation.as_ref()
    }

    pub fn outcome_class(&self) -> &OutcomeClass {
        &self.outcome_class
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    pub fn final_budget(&self) -> SessionBudget {
        self.final_budget
    }
}
