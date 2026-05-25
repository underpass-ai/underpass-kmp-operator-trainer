use operator_shared_domain::action::operator_action::OperatorAction;

use crate::session::observation::Observation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStep {
    action: OperatorAction,
    observation: Observation,
}

impl ExecutionStep {
    pub fn new(action: OperatorAction, observation: Observation) -> Self {
        Self {
            action,
            observation,
        }
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }

    pub fn observation(&self) -> &Observation {
        &self.observation
    }
}
