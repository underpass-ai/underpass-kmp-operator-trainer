//! Default `ActionContractValidator`. Walks a fixed sequence of
//! specifications, accumulating every violation rather than failing fast.
//! Dataset and replay reports benefit from the complete list.

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::action_contract_validator::ActionContractValidator;
use crate::contract::contract_violations::ContractViolations;
use crate::mode::operator_mode::OperatorMode;
use crate::specifications::arguments_reference_known_entities_spec::ArgumentsReferenceKnownEntitiesSpec;
use crate::specifications::budget_allows_action_spec::BudgetAllowsActionSpec;
use crate::specifications::cursor_reachable_from_visible_spec::CursorReachableFromVisibleSpec;
use crate::specifications::specification::Specification;
use crate::specifications::tool_within_mode_spec::ToolWithinModeSpec;
use crate::visible_state::visible_state::VisibleState;

pub struct CompositeActionContractValidator {
    specs: Vec<Box<dyn for<'a> Specification<ActionContractSubject<'a>>>>,
}

impl std::fmt::Debug for CompositeActionContractValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeActionContractValidator")
            .field("specs_len", &self.specs.len())
            .finish()
    }
}

impl CompositeActionContractValidator {
    /// Construct an explicit composite. Reserved for testing and for
    /// alternative validator strategies; production code calls
    /// [`Self::default_strict`].
    pub fn new(specs: Vec<Box<dyn for<'a> Specification<ActionContractSubject<'a>>>>) -> Self {
        Self { specs }
    }

    /// Construct the strict production validator: every shared-context
    /// specification, evaluated in order.
    pub fn default_strict() -> Self {
        Self::new(vec![
            Box::new(ToolWithinModeSpec::new()),
            Box::new(ArgumentsReferenceKnownEntitiesSpec::new()),
            Box::new(CursorReachableFromVisibleSpec::new()),
            Box::new(BudgetAllowsActionSpec::new()),
        ])
    }
}

impl ActionContractValidator for CompositeActionContractValidator {
    fn validate(
        &self,
        action: &OperatorAction,
        mode: OperatorMode,
        visible: &VisibleState,
    ) -> Result<(), ContractViolations> {
        let subject = ActionContractSubject::new(action, mode, visible);
        let mut violations = ContractViolations::new();
        for spec in &self.specs {
            if let Err(violation) = spec.evaluate(&subject) {
                violations.push(violation);
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}
