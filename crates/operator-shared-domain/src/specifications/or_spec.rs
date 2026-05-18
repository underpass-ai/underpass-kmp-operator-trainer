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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::contract_violation_code::ContractViolationCode;

    #[derive(Debug)]
    struct AlwaysOk;
    impl Specification<()> for AlwaysOk {
        fn evaluate(&self, (): &()) -> Result<(), ContractViolation> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct AlwaysFails(&'static str);
    impl Specification<()> for AlwaysFails {
        fn evaluate(&self, (): &()) -> Result<(), ContractViolation> {
            Err(ContractViolation::new(
                ContractViolationCode::BudgetExhausted,
                "f",
                self.0,
            ))
        }
    }

    #[test]
    fn ok_when_first_succeeds() {
        let spec = OrSpec::new(Box::new(AlwaysOk), Box::new(AlwaysFails("never reached")));
        assert!(spec.evaluate(&()).is_ok());
        assert!(format!("{spec:?}").contains("OrSpec"));
    }

    #[test]
    fn ok_when_first_fails_but_second_succeeds() {
        let spec = OrSpec::new(Box::new(AlwaysFails("ignored")), Box::new(AlwaysOk));
        assert!(spec.evaluate(&()).is_ok());
    }

    #[test]
    fn returns_second_violation_when_both_fail() {
        let spec = OrSpec::new(
            Box::new(AlwaysFails("ignored")),
            Box::new(AlwaysFails("specific")),
        );
        let err = spec.evaluate(&()).unwrap_err();
        assert_eq!(err.message(), "specific");
    }
}
