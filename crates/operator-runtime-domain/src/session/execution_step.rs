use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::session::observation::Observation;

/// One predict + execute iteration of a multi-step session: the action the
/// policy chose, the observation the executor returned, and the
/// `perceived_state` the policy actually saw when it chose that action.
///
/// `perceived_state` is the visible state the loop built the subject from
/// *before* folding this step's observation back in. Recording it makes the
/// transcript a complete decision log: a downstream trainer can pair
/// `(perceived_state, action)` directly as an SFT `(prompt, target)` example
/// without replaying the loop's state threading (which would risk an off-by-one
/// between the state before vs. after the step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStep {
    action: OperatorAction,
    observation: Observation,
    perceived_state: VisibleState,
}

impl ExecutionStep {
    pub fn new(
        action: OperatorAction,
        observation: Observation,
        perceived_state: VisibleState,
    ) -> Self {
        Self {
            action,
            observation,
            perceived_state,
        }
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }

    pub fn observation(&self) -> &Observation {
        &self.observation
    }

    /// The visible state the policy perceived when it chose `action` (before
    /// this step's observation was folded into the next state).
    pub fn perceived_state(&self) -> &VisibleState {
        &self.perceived_state
    }
}
