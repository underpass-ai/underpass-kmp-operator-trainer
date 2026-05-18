//! Composite specification that succeeds when either sub-specification
//! succeeds. On failure it returns the violation reported by the second
//! specification, which is treated as the more specific rule.

use crate::contract::contract_violation::ContractViolation;
use crate::specifications::specification::Specification;

pub struct OrSpec<T: ?Sized> {
    first: Box<dyn Specification<T>>,
    second: Box<dyn Specification<T>>,
}

impl<T: ?Sized> OrSpec<T> {
    pub fn new(first: Box<dyn Specification<T>>, second: Box<dyn Specification<T>>) -> Self {
        Self { first, second }
    }
}

impl<T: ?Sized> std::fmt::Debug for OrSpec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrSpec")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<T: ?Sized> Specification<T> for OrSpec<T> {
    fn evaluate(&self, subject: &T) -> Result<(), ContractViolation> {
        if self.first.evaluate(subject).is_ok() {
            return Ok(());
        }
        self.second.evaluate(subject)
    }
}
