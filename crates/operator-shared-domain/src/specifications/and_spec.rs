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
    fn ok_when_both_succeed() {
        let spec = AndSpec::new(Box::new(AlwaysOk), Box::new(AlwaysOk));
        assert!(spec.evaluate(&()).is_ok());
        // Debug impl exercised
        assert!(format!("{spec:?}").contains("AndSpec"));
    }

    #[test]
    fn fails_with_first_violation_when_first_fails() {
        let spec = AndSpec::new(
            Box::new(AlwaysFails("first")),
            Box::new(AlwaysFails("second")),
        );
        let err = spec.evaluate(&()).unwrap_err();
        assert_eq!(err.message(), "first");
    }

    #[test]
    fn fails_with_second_when_first_passes_and_second_fails() {
        let spec = AndSpec::new(Box::new(AlwaysOk), Box::new(AlwaysFails("second")));
        let err = spec.evaluate(&()).unwrap_err();
        assert_eq!(err.message(), "second");
    }
}
