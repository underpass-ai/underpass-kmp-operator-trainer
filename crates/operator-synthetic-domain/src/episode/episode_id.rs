//! Stable identifier for one synthetic episode.

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpisodeId {
    inner: NonEmptyString,
}

impl EpisodeId {
    pub fn parse(value: impl Into<String>) -> SyntheticDomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "episode_id")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for EpisodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_id() {
        let id = EpisodeId::parse("episode:1").unwrap();
        assert_eq!(id.as_str(), "episode:1");
        assert_eq!(id.clone(), id);
    }

    #[test]
    fn rejects_empty_id() {
        assert!(EpisodeId::parse(" ").is_err());
    }
}
