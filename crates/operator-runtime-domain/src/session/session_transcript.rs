use std::time::Duration;

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::budget::session_budget::SessionBudget;
use crate::session::execution_step::ExecutionStep;
use crate::session::operator_session_id::OperatorSessionId;
use crate::session::outcome_class::OutcomeClass;

/// Result of a multi-step operator session.
///
/// Where [`crate::session::session_outcome::SessionOutcome`] describes a single
/// predict/execute, a transcript accumulates every [`ExecutionStep`] the loop
/// ran (one per executed tool call) and records why the session ended
/// (`outcome_class`), the terminal action that ended it, the final visible
/// state the policy reached, and the budget left.
///
/// Tool errors are recorded as steps and fed back to the policy, so they are
/// not terminal; a session ends only on stop, escalate, budget exhaustion, or a
/// contract violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTranscript {
    session_id: OperatorSessionId,
    steps: Vec<ExecutionStep>,
    terminal_action: OperatorAction,
    outcome_class: OutcomeClass,
    final_visible_state: VisibleState,
    final_budget: SessionBudget,
    elapsed_ms: u64,
}

impl SessionTranscript {
    /// The policy chose to stop: enough evidence was gathered.
    pub fn completed(
        session_id: OperatorSessionId,
        steps: Vec<ExecutionStep>,
        terminal_action: OperatorAction,
        final_visible_state: VisibleState,
        final_budget: SessionBudget,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::Completed,
            final_visible_state,
            final_budget,
            elapsed,
        )
    }

    /// The policy escalated: the next decision needs open reasoning, not memory
    /// navigation.
    pub fn escalated(
        session_id: OperatorSessionId,
        steps: Vec<ExecutionStep>,
        terminal_action: OperatorAction,
        final_visible_state: VisibleState,
        final_budget: SessionBudget,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::Escalated,
            final_visible_state,
            final_budget,
            elapsed,
        )
    }

    /// The policy wanted another tool call but the call budget was exhausted.
    pub fn budget_exhausted(
        session_id: OperatorSessionId,
        steps: Vec<ExecutionStep>,
        terminal_action: OperatorAction,
        final_visible_state: VisibleState,
        final_budget: SessionBudget,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::BudgetExhausted,
            final_visible_state,
            final_budget,
            elapsed,
        )
    }

    /// A predicted action violated the operator action contract.
    #[allow(clippy::too_many_arguments)]
    pub fn contract_violation(
        session_id: OperatorSessionId,
        steps: Vec<ExecutionStep>,
        terminal_action: OperatorAction,
        violations: ContractViolations,
        final_visible_state: VisibleState,
        final_budget: SessionBudget,
        elapsed: Duration,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::ContractViolation { violations },
            final_visible_state,
            final_budget,
            elapsed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        session_id: OperatorSessionId,
        steps: Vec<ExecutionStep>,
        terminal_action: OperatorAction,
        outcome_class: OutcomeClass,
        final_visible_state: VisibleState,
        final_budget: SessionBudget,
        elapsed: Duration,
    ) -> Self {
        Self {
            session_id,
            steps,
            terminal_action,
            outcome_class,
            final_visible_state,
            final_budget,
            elapsed_ms: elapsed.as_millis().try_into().unwrap_or(u64::MAX),
        }
    }

    pub fn session_id(&self) -> &OperatorSessionId {
        &self.session_id
    }

    pub fn steps(&self) -> &[ExecutionStep] {
        &self.steps
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn terminal_action(&self) -> &OperatorAction {
        &self.terminal_action
    }

    pub fn outcome_class(&self) -> &OutcomeClass {
        &self.outcome_class
    }

    pub fn final_visible_state(&self) -> &VisibleState {
        &self.final_visible_state
    }

    pub fn final_budget(&self) -> SessionBudget {
        self.final_budget
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }
}
