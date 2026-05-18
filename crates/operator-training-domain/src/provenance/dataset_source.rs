//! Where a training dataset came from, captured as an opaque
//! identifier (typically a synthetic run id, a git SHA, or a path).
//! Kept stringly because the source registry is not part of this
//! context's domain — it is a label the consumer of the manifest can
//! trace back.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DatasetSource {
    inner: NonEmptyString,
}

impl DatasetSource {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "dataset_source")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for DatasetSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_source() {
        assert!(DatasetSource::parse("").is_err());
    }

    #[test]
    fn round_trips_label() {
        let src = DatasetSource::parse("synthetic-run:2026-05-18").unwrap();
        assert_eq!(src.as_str(), "synthetic-run:2026-05-18");
    }
}
