//! Accepted action set for one calibration case.

use operator_shared_domain::action::operator_action::OperatorAction;

use crate::calibration::calibration_capability::CalibrationCapability;
use crate::error::synthetic_domain_error::SyntheticDomainError;
use crate::error::synthetic_domain_result::SyntheticDomainResult;

const MAX_ACCEPTED_ACTIONS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedActions {
    actions: Vec<OperatorAction>,
    capability: CalibrationCapability,
}

impl AcceptedActions {
    pub fn new(actions: Vec<OperatorAction>) -> SyntheticDomainResult<Self> {
        let Some(first) = actions.first() else {
            return Err(SyntheticDomainError::EmptyAcceptedActions);
        };
        if actions.len() > MAX_ACCEPTED_ACTIONS {
            return Err(SyntheticDomainError::TooManyAcceptedActions {
                maximum: MAX_ACCEPTED_ACTIONS,
                actual: actions.len(),
            });
        }
        let capability = CalibrationCapability::from_action(first);
        for action in &actions {
            let actual = CalibrationCapability::from_action(action);
            if actual != capability {
                return Err(SyntheticDomainError::MixedAcceptedActionCapability {
                    expected: capability.as_str().to_string(),
                    actual: actual.as_str().to_string(),
                });
            }
        }
        Ok(Self {
            actions,
            capability,
        })
    }

    pub fn contains(&self, action: &OperatorAction) -> bool {
        self.actions.contains(action)
    }

    pub fn capability(&self) -> CalibrationCapability {
        self.capability
    }

    pub fn as_slice(&self) -> &[OperatorAction] {
        &self.actions
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;

    #[test]
    fn accepts_one_to_three_actions_with_same_capability() {
        let action = inspect("node:1");
        let accepted = AcceptedActions::new(vec![action.clone(), inspect("node:2")]).unwrap();
        assert!(accepted.contains(&action));
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted.capability(), CalibrationCapability::KernelInspect);
    }

    #[test]
    fn rejects_empty_actions() {
        assert!(matches!(
            AcceptedActions::new(vec![]),
            Err(SyntheticDomainError::EmptyAcceptedActions)
        ));
    }

    #[test]
    fn rejects_mixed_capabilities() {
        let stop =
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap());
        assert!(matches!(
            AcceptedActions::new(vec![inspect("node:1"), stop]),
            Err(SyntheticDomainError::MixedAcceptedActionCapability { .. })
        ));
    }

    fn inspect(target: &str) -> OperatorAction {
        OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse(target).unwrap()),
        )))
    }
}
