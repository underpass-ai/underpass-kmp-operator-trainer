use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::correctness::correctness_mode::CorrectnessMode;
use crate::contract::correctness::field_outcome::FieldOutcome;
use crate::contract::correctness::field_path::FieldPath;
use crate::contract::correctness::field_result::FieldResult;
use crate::value_objects::non_empty_string::NonEmptyString;

pub fn field_result_exact(path: &'static str, actual: String, expected: String) -> FieldResult {
    let outcome = if actual == expected {
        FieldOutcome::Pass
    } else {
        FieldOutcome::Fail { expected, actual }
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::Exact,
        outcome,
    )
}

pub fn field_result_exact_bool(path: &'static str, actual: bool, expected: bool) -> FieldResult {
    field_result_exact(path, actual.to_string(), expected.to_string())
}

pub fn field_result_exact_debug<T: std::fmt::Debug>(
    path: &'static str,
    actual: &T,
    expected: &T,
) -> FieldResult {
    field_result_exact(path, format!("{actual:?}"), format!("{expected:?}"))
}

pub fn field_result_schema_valid_non_empty(
    path: &'static str,
    actual: &NonEmptyString,
) -> FieldResult {
    let outcome = if actual.as_str().trim().is_empty() {
        FieldOutcome::SchemaInvalid {
            message: "expected non-empty string".to_string(),
        }
    } else {
        FieldOutcome::Pass
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::SchemaValid,
        outcome,
    )
}

pub fn field_result_schema_valid_optional_non_empty(
    path: &'static str,
    actual: Option<&NonEmptyString>,
    expected: Option<&NonEmptyString>,
) -> FieldResult {
    let outcome = match (actual, expected) {
        (None, None) => FieldOutcome::Pass,
        (Some(value), _) if !value.as_str().trim().is_empty() => FieldOutcome::Pass,
        (Some(_), _) => FieldOutcome::SchemaInvalid {
            message: "expected non-empty string".to_string(),
        },
        (None, Some(_)) => FieldOutcome::SchemaInvalid {
            message: "expected field to be present".to_string(),
        },
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::SchemaValid,
        outcome,
    )
}

pub fn field_result_schema_valid_uuid(path: &'static str, actual: &str) -> FieldResult {
    let outcome = if is_uuid(actual) {
        FieldOutcome::Pass
    } else {
        FieldOutcome::SchemaInvalid {
            message: format!("expected UUID, got {actual}"),
        }
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::SchemaValid,
        outcome,
    )
}

pub fn field_result_schema_valid_system_time(
    path: &'static str,
    actual: Option<SystemTime>,
    expected: Option<SystemTime>,
) -> FieldResult {
    let outcome = match (actual, expected) {
        (Some(_), _) | (None, None) => FieldOutcome::Pass,
        (None, Some(_)) => FieldOutcome::SchemaInvalid {
            message: "expected timestamp to be present".to_string(),
        },
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::SchemaValid,
        outcome,
    )
}

pub fn field_result_permissive_optional(
    path: &'static str,
    actual: Option<&NonEmptyString>,
    expected: Option<&NonEmptyString>,
) -> FieldResult {
    let outcome = match (actual, expected) {
        (None, None) => FieldOutcome::Pass,
        (Some(value), _) if !value.as_str().trim().is_empty() => FieldOutcome::Pass,
        (Some(_), _) => FieldOutcome::SchemaInvalid {
            message: "expected non-empty free text".to_string(),
        },
        (None, Some(_)) => FieldOutcome::SchemaInvalid {
            message: "expected free-text field to be present".to_string(),
        },
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::Permissive,
        outcome,
    )
}

pub fn field_result_permissive_required(
    path: &'static str,
    actual: &NonEmptyString,
) -> FieldResult {
    let outcome = if actual.as_str().trim().is_empty() {
        FieldOutcome::SchemaInvalid {
            message: "expected non-empty free text".to_string(),
        }
    } else {
        FieldOutcome::Pass
    };
    FieldResult::new(
        FieldPath::trusted_static(path),
        CorrectnessMode::Permissive,
        outcome,
    )
}

pub fn system_time_debug(value: Option<SystemTime>) -> String {
    value.map_or_else(
        || "<none>".to_string(),
        |time| match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => format!("unix:{}.{:09}", duration.as_secs(), duration.subsec_nanos()),
            Err(err) => format!(
                "before_unix:{}.{:09}",
                err.duration().as_secs(),
                err.duration().subsec_nanos()
            ),
        },
    )
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_validator_is_strict() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!is_uuid("idem:abc"));
    }
}
