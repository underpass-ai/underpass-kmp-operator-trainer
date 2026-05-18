//! Composite specification that succeeds only when both sub-specifications
//! succeed. On failure it returns the first violation encountered, in
//! left-to-right order.

use crate::contract::contract_violation::ContractViolation;
use crate::specifications::specification::Specification;

pub struct AndSpec<T: ?Sized> {
    first: Box<dyn Specification<T>>,
    second: Box<dyn Specification<T>>,
}

impl<T: ?Sized> AndSpec<T> {
    pub fn new(first: Box<dyn Specification<T>>, second: Box<dyn Specification<T>>) -> Self {
        Self { first, second }
    }
}

impl<T: ?Sized> std::fmt::Debug for AndSpec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AndSpec")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<T: ?Sized> Specification<T> for AndSpec<T> {
    fn evaluate(&self, subject: &T) -> Result<(), ContractViolation> {
        self.first.evaluate(subject)?;
        self.second.evaluate(subject)
    }
}
