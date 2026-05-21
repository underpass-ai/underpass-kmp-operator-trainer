//! Calibration case category: normal policy path or adversarial policy trap.

use operator_shared_domain::error::domain_error::DomainError;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CalibrationCaseCategory {
    Happy,
    Adversarial,
}

impl CalibrationCaseCategory {
    pub const ALL: [Self; 2] = [Self::Happy, Self::Adversarial];

    pub fn parse(value: &str) -> SyntheticDomainResult<Self> {
        match value {
            "happy" => Ok(Self::Happy),
            "adversarial" => Ok(Self::Adversarial),
            other => Err(DomainError::UnsupportedValue {
                context: "calibration_case_category",
                value: other.to_string(),
            }
            .into()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Happy => "happy",
            Self::Adversarial => "adversarial",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_categories() {
        assert_eq!(
            CalibrationCaseCategory::parse("happy").unwrap(),
            CalibrationCaseCategory::Happy
        );
        assert_eq!(CalibrationCaseCategory::Adversarial.as_str(), "adversarial");
    }

    #[test]
    fn rejects_unknown_category() {
        assert!(CalibrationCaseCategory::parse("neutral").is_err());
    }
}
