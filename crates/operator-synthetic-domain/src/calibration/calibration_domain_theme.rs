//! Closed theme vocabulary for teacher calibration cases.

use operator_shared_domain::error::domain_error::DomainError;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CalibrationDomainTheme {
    TechnicalIncident,
    SoftwareMigration,
    BugInvestigation,
    ProductPlanning,
    SmartWritingSession,
}

impl CalibrationDomainTheme {
    pub fn parse(value: &str) -> SyntheticDomainResult<Self> {
        match value {
            "technical_incident" => Ok(Self::TechnicalIncident),
            "software_migration" => Ok(Self::SoftwareMigration),
            "bug_investigation" => Ok(Self::BugInvestigation),
            "product_planning" => Ok(Self::ProductPlanning),
            "smart_writing_session" => Ok(Self::SmartWritingSession),
            other => Err(DomainError::UnsupportedValue {
                context: "calibration_domain_theme",
                value: other.to_string(),
            }
            .into()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TechnicalIncident => "technical_incident",
            Self::SoftwareMigration => "software_migration",
            Self::BugInvestigation => "bug_investigation",
            Self::ProductPlanning => "product_planning",
            Self::SmartWritingSession => "smart_writing_session",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_theme() {
        assert_eq!(
            CalibrationDomainTheme::parse("technical_incident").unwrap(),
            CalibrationDomainTheme::TechnicalIncident
        );
        assert_eq!(
            CalibrationDomainTheme::SmartWritingSession.as_str(),
            "smart_writing_session"
        );
    }

    #[test]
    fn rejects_unknown_theme() {
        assert!(CalibrationDomainTheme::parse("marketing").is_err());
    }
}
