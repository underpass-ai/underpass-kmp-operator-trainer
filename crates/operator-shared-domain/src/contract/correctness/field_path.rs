use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldPath {
    raw: String,
}

impl FieldPath {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        let raw = value.into();
        if raw.trim().is_empty() {
            return Err(DomainError::EmptyValue {
                context: "field_path",
            });
        }
        if !is_valid_field_path(&raw) {
            return Err(DomainError::UnsupportedValue {
                context: "field_path",
                value: raw,
            });
        }
        Ok(Self { raw })
    }

    pub(crate) fn trusted_static(value: &'static str) -> Self {
        debug_assert!(is_valid_field_path(value));
        Self {
            raw: value.to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

fn is_valid_field_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !is_field_start(bytes[0]) {
        return false;
    }

    let mut index = consume_field_segment(bytes, 0);
    while index < bytes.len() {
        match bytes[index] {
            b'.' => {
                index += 1;
                if index >= bytes.len() || !is_field_start(bytes[index]) {
                    return false;
                }
                index = consume_field_segment(bytes, index);
            }
            b'[' => {
                index += 1;
                if index >= bytes.len() {
                    return false;
                }
                if bytes[index] == b'*' {
                    index += 1;
                } else {
                    let start = index;
                    while index < bytes.len() && bytes[index].is_ascii_digit() {
                        index += 1;
                    }
                    if index == start {
                        return false;
                    }
                }
                if index >= bytes.len() || bytes[index] != b']' {
                    return false;
                }
                index += 1;
            }
            _ => return false,
        }
    }
    true
}

fn consume_field_segment(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && is_field_char(bytes[index]) {
        index += 1;
    }
    index
}

fn is_field_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_lowercase()
}

fn is_field_char(byte: u8) -> bool {
    is_field_start(byte) || byte.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_paths() {
        for path in [
            "kind",
            "provenance.observed_at",
            "entries[2].ref",
            "dimensions[*]",
            "memory.entries[*].coordinates[*].sequence",
        ] {
            assert_eq!(FieldPath::parse(path).unwrap().as_str(), path);
        }
    }

    #[test]
    fn rejects_invalid_paths() {
        for path in ["", "Field", "entries[]", "entries[abc]", ".kind", "kind."] {
            assert!(FieldPath::parse(path).is_err(), "{path} should fail");
        }
    }
}
