//! Human-readable objective that every step in an episode serves.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpisodeObjective {
    inner: NonEmptyString,
}

impl EpisodeObjective {
    pub fn parse(value: impl Into<String>) -> SyntheticDomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "episode_objective")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for EpisodeObjective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_objective() {
        let objective = EpisodeObjective::parse("Find the safe deployment path.").unwrap();
        assert_eq!(objective.as_str(), "Find the safe deployment path.");
        assert_eq!(objective.clone(), objective);
    }

    #[test]
    fn rejects_blank_objective() {
        assert!(EpisodeObjective::parse("").is_err());
    }
}
