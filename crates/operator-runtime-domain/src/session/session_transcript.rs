use std::time::Duration;

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::contract_violations::ContractViolations;
use operator_shared_domain::ids::about_id::AboutId;
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
    terminal_about: AboutId,
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
        terminal_about: AboutId,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::Completed,
            final_visible_state,
            final_budget,
            elapsed,
            terminal_about,
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
        terminal_about: AboutId,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::Escalated,
            final_visible_state,
            final_budget,
            elapsed,
            terminal_about,
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
        terminal_about: AboutId,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::BudgetExhausted,
            final_visible_state,
            final_budget,
            elapsed,
            terminal_about,
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
        terminal_about: AboutId,
    ) -> Self {
        Self::new(
            session_id,
            steps,
            terminal_action,
            OutcomeClass::ContractViolation { violations },
            final_visible_state,
            final_budget,
            elapsed,
            terminal_about,
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
        terminal_about: AboutId,
    ) -> Self {
        Self {
            session_id,
            steps,
            terminal_action,
            outcome_class,
            final_visible_state,
            final_budget,
            elapsed_ms: elapsed.as_millis().try_into().unwrap_or(u64::MAX),
            terminal_about,
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

    /// The about current at the terminal decision — for a cross-about session,
    /// the about the last `kernel_wake` switched to. Lets the terminal SFT row
    /// be attributed to the about the stop/escalate was reasoning over.
    pub fn terminal_about(&self) -> &AboutId {
        &self.terminal_about
    }
}
