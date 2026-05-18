//! Subject of action contract validation. Three references the
//! specifications need: the candidate `OperatorAction`, the current
//! `OperatorMode`, and the `VisibleState` snapshot.

use crate::action::operator_action::OperatorAction;
use crate::mode::operator_mode::OperatorMode;
use crate::visible_state::visible_state::VisibleState;

#[derive(Debug, Clone, Copy)]
pub struct ActionContractSubject<'a> {
    action: &'a OperatorAction,
    mode: OperatorMode,
    visible: &'a VisibleState,
}

impl<'a> ActionContractSubject<'a> {
    pub fn new(action: &'a OperatorAction, mode: OperatorMode, visible: &'a VisibleState) -> Self {
        Self {
            action,
            mode,
            visible,
        }
    }

    pub fn action(&self) -> &OperatorAction {
        self.action
    }

    pub fn mode(&self) -> OperatorMode {
        self.mode
    }

    pub fn visible(&self) -> &VisibleState {
        self.visible
    }
}
