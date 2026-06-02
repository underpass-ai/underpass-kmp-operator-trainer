use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::session::observation::Observation;

/// One predict + execute iteration of a multi-step session: the action the
/// policy chose, the observation the executor returned, the `perceived_state`
/// the policy actually saw when it chose that action, and the `current_about`
/// that scoped the step.
///
/// `perceived_state` is the visible state the loop built the subject from
/// *before* folding this step's observation back in. Recording it makes the
/// transcript a complete decision log: a downstream trainer can pair
/// `(perceived_state, action)` directly as an SFT `(prompt, target)` example
/// without replaying the loop's state threading (which would risk an off-by-one
/// between the state before vs. after the step).
///
/// `current_about` is the about that was current when the policy chose this
/// step — its memory namespace. In a single-about session it equals the
/// request's about on every step; in a cross-about session it is the about the
/// last `kernel_wake` switched to, so downstream SFT rows can be attributed to
/// the about the step was actually reasoning over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStep {
    action: OperatorAction,
    observation: Observation,
    perceived_state: VisibleState,
    current_about: AboutId,
}

impl ExecutionStep {
    pub fn new(
        action: OperatorAction,
        observation: Observation,
        perceived_state: VisibleState,
        current_about: AboutId,
    ) -> Self {
        Self {
            action,
            observation,
            perceived_state,
            current_about,
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

    /// The about that was current when the policy chose this step (the step's
    /// memory namespace).
    pub fn about(&self) -> &AboutId {
        &self.current_about
    }
}
