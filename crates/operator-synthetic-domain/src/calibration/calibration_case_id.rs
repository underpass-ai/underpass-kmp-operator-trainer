//! Stable identifier for a teacher calibration case.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalibrationCaseId {
    value: NonEmptyString,
}

impl CalibrationCaseId {
    pub fn parse(value: impl Into<String>) -> SyntheticDomainResult<Self> {
        Ok(Self {
            value: NonEmptyString::parse(value, "calibration_case_id")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl std::fmt::Display for CalibrationCaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_id() {
        let id = CalibrationCaseId::parse("calib:case").unwrap();
        assert_eq!(id.as_str(), "calib:case");
    }

    #[test]
    fn rejects_empty_id() {
        assert!(CalibrationCaseId::parse("").is_err());
    }
}
