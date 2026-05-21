//! Human-facing explanation for the accepted calibration action.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedActionRationale {
    value: NonEmptyString,
}

impl ExpectedActionRationale {
    pub fn parse(value: impl Into<String>) -> SyntheticDomainResult<Self> {
        Ok(Self {
            value: NonEmptyString::parse(value, "expected_action_rationale")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_rationale() {
        let rationale = ExpectedActionRationale::parse("Inspect is cheapest.").unwrap();
        assert_eq!(rationale.as_str(), "Inspect is cheapest.");
    }

    #[test]
    fn rejects_empty_rationale() {
        assert!(ExpectedActionRationale::parse("").is_err());
    }
}
