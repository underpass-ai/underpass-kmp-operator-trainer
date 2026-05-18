//! Cryptographic content hash of a serialised dataset. Stored as a
//! prefixed string (e.g., `"sha256:..."`) so the algorithm is visible
//! in the manifest. The parser enforces the `<algorithm>:<hex>` shape
//! at the boundary:
//!
//! - the prefix before the colon must be non-empty and made of
//!   lowercase ASCII alphanumerics plus `_` / `-` (matches common
//!   algorithm labels: `sha256`, `sha512`, `blake3`, `sha2_256`, …),
//! - the suffix after the colon must be non-empty and made entirely
//!   of lowercase ASCII hexadecimal digits.
//!
//! The value object does **not** verify that the hex digest length
//! matches the algorithm: verification (e.g., that `sha256:` carries
//! exactly 64 hex digits) is the consumer's responsibility. The
//! domain models the shape, not the per-algorithm semantics.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash {
    inner: NonEmptyString,
}

impl ContentHash {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        let raw_string = value.into();
        let inner = NonEmptyString::parse(raw_string.clone(), "content_hash")?;
        validate_shape(inner.as_str()).map_err(|reason| DomainError::UnsupportedValue {
            context: "content_hash",
            value: format!("{raw_string}: {reason}"),
        })?;
        Ok(Self { inner })
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

fn validate_shape(raw: &str) -> Result<(), &'static str> {
    let (algorithm, digest) = raw.split_once(':').ok_or("missing ':' separator")?;
    if algorithm.is_empty() {
        return Err("algorithm prefix is empty");
    }
    if !algorithm
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err("algorithm prefix must be lowercase alphanumeric (with `_` or `-`)");
    }
    if digest.is_empty() {
        return Err("hex digest is empty");
    }
    if !digest
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("hex digest must be lowercase hexadecimal");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_hash() {
        assert!(ContentHash::parse("").is_err());
    }

    #[test]
    fn accepts_prefixed_sha256() {
        let hash = ContentHash::parse("sha256:0123456789abcdef").unwrap();
        assert_eq!(hash.as_str(), "sha256:0123456789abcdef");
    }

    #[test]
    fn accepts_other_algorithm_labels() {
        // The domain does not lock the algorithm; sha512, blake3, etc.
        // are equally valid as long as the shape holds.
        ContentHash::parse("sha512:cafe").unwrap();
        ContentHash::parse("blake3:0011").unwrap();
        ContentHash::parse("sha2_256:abcd").unwrap();
    }

    #[test]
    fn refuses_missing_colon() {
        let err = ContentHash::parse("sha256abcdef").unwrap_err();
        assert!(matches!(
            err,
            DomainError::UnsupportedValue {
                context: "content_hash",
                ..
            }
        ));
    }

    #[test]
    fn refuses_empty_algorithm_prefix() {
        assert!(ContentHash::parse(":abcdef").is_err());
    }

    #[test]
    fn refuses_empty_digest() {
        assert!(ContentHash::parse("sha256:").is_err());
    }

    #[test]
    fn refuses_non_hex_digest() {
        assert!(ContentHash::parse("sha256:zzzz").is_err());
    }

    #[test]
    fn refuses_uppercase_hex_digest() {
        // Keep the shape canonical — uppercase hex would round-trip
        // unequal to the lowercase hashes emitted by the writer.
        assert!(ContentHash::parse("sha256:ABCDEF").is_err());
    }

    #[test]
    fn refuses_uppercase_algorithm() {
        assert!(ContentHash::parse("SHA256:abcdef").is_err());
    }

    #[test]
    fn refuses_garbage_string() {
        assert!(ContentHash::parse("garbage").is_err());
    }
}
