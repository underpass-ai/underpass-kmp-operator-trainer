//! Cryptographic content hash of a serialised dataset. Stored as a
//! prefixed string (e.g., `"sha256:..."`) so the algorithm is visible
//! in the manifest. This domain refuses to bake in a specific algorithm
//! — verification is the consumer's job.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash {
    inner: NonEmptyString,
}

impl ContentHash {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "content_hash")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_hash() {
        assert!(ContentHash::parse("").is_err());
    }

    #[test]
    fn accepts_prefixed_hash() {
        let hash = ContentHash::parse("sha256:0123456789abcdef").unwrap();
        assert_eq!(hash.as_str(), "sha256:0123456789abcdef");
    }
}
