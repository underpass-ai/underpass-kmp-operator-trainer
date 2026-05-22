//! SHA-256 hex digest for the exact teacher-facing subject payload.

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectHash {
    raw: String,
}

impl SubjectHash {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        let raw = value.into();
        if raw.len() != 64
            || !raw
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
        {
            return Err(DomainError::UnsupportedValue {
                context: "subject_hash",
                value: raw,
            });
        }
        Ok(Self { raw })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl std::fmt::Display for SubjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lowercase_sha256_hex() {
        let hash =
            SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .expect("valid hash parses");
        assert_eq!(
            hash.as_str(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn rejects_short_value() {
        assert!(SubjectHash::parse("abc").is_err());
    }

    #[test]
    fn rejects_uppercase_hex() {
        assert!(
            SubjectHash::parse("0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef")
                .is_err()
        );
    }
}
