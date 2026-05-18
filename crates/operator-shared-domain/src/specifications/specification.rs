//! Generic Specification trait. A specification answers "does this subject
//! satisfy a domain rule?" and returns either success or a structured
//! `ContractViolation`.
//!
//! Concrete specifications target a specific subject type. They are
//! composed via `AndSpec` and `OrSpec` combinators that live in dedicated
//! files in this module.

use crate::contract::contract_violation::ContractViolation;

pub trait Specification<T: ?Sized>: std::fmt::Debug + Send + Sync {
    fn evaluate(&self, subject: &T) -> Result<(), ContractViolation>;
}
