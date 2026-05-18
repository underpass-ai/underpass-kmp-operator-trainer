//! Trait that wraps a collection of specifications and exposes a single
//! `validate(...)` method to higher layers. The default production
//! implementation is [`CompositeActionContractValidator`].

use crate::action::operator_action::OperatorAction;
use crate::contract::contract_violations::ContractViolations;
use crate::mode::operator_mode::OperatorMode;
use crate::visible_state::visible_state::VisibleState;

pub trait ActionContractValidator: std::fmt::Debug + Send + Sync {
    fn validate(
        &self,
        action: &OperatorAction,
        mode: OperatorMode,
        visible: &VisibleState,
    ) -> Result<(), ContractViolations>;
}
