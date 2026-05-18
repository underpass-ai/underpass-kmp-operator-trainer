use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PositiveCount {
    raw: usize,
}

impl PositiveCount {
    pub fn parse(value: usize, context: &'static str) -> DomainResult<Self> {
        if value == 0 {
            return Err(DomainError::NonPositiveCount { context });
        }
        Ok(Self { raw: value })
    }

    pub fn as_usize(self) -> usize {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_zero() {
        assert_eq!(
            PositiveCount::parse(0, "test"),
            Err(DomainError::NonPositiveCount { context: "test" })
        );
    }

    #[test]
    fn accepts_positive() {
        assert_eq!(PositiveCount::parse(3, "test").unwrap().as_usize(), 3);
    }
}
