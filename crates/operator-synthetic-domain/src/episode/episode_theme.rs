//! Closed taxonomy for realistic synthetic episode themes.

use operator_shared_domain::error::domain_error::DomainError;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpisodeTheme {
    Incident,
    BugInvestigation,
    Migration,
    ProductDecision,
    MemoryTask,
    SmartWritingSession,
    TemporalWindowExpansion,
}

impl EpisodeTheme {
    pub fn parse(value: &str) -> SyntheticDomainResult<Self> {
        match value {
            "incident" => Ok(Self::Incident),
            "bug_investigation" => Ok(Self::BugInvestigation),
            "migration" => Ok(Self::Migration),
            "product_decision" => Ok(Self::ProductDecision),
            "memory_task" => Ok(Self::MemoryTask),
            "smart_writing_session" => Ok(Self::SmartWritingSession),
            "temporal_window_expansion" => Ok(Self::TemporalWindowExpansion),
            other => Err(DomainError::UnsupportedValue {
                context: "episode_theme",
                value: other.to_string(),
            }
            .into()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Incident => "incident",
            Self::BugInvestigation => "bug_investigation",
            Self::Migration => "migration",
            Self::ProductDecision => "product_decision",
            Self::MemoryTask => "memory_task",
            Self::SmartWritingSession => "smart_writing_session",
            Self::TemporalWindowExpansion => "temporal_window_expansion",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_theme() {
        assert_eq!(
            EpisodeTheme::parse("incident").unwrap(),
            EpisodeTheme::Incident
        );
        assert_eq!(EpisodeTheme::Incident.as_str(), "incident");
    }

    #[test]
    fn rejects_unknown_theme() {
        assert!(EpisodeTheme::parse("marketing").is_err());
    }

    #[test]
    fn round_trips_temporal_window_expansion() {
        let theme = EpisodeTheme::parse("temporal_window_expansion").unwrap();
        assert_eq!(theme, EpisodeTheme::TemporalWindowExpansion);
        assert_eq!(theme.as_str(), "temporal_window_expansion");
    }
}
