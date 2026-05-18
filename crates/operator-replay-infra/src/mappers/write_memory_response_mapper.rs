//! Map the structured content of a `kernel_write_memory` response to
//! a typed `WriteMemoryOutcome`. First-pass capture: `summary`,
//! `accepted`, `dry_run`, `generated_refs`. The relation-quality
//! diagnostics and ingest preview are dropped.

use operator_shared_domain::tool_outcomes::write_memory_outcome::WriteMemoryOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "write_memory";

#[derive(Debug)]
pub struct WriteMemoryResponseMapper;

impl WriteMemoryResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<WriteMemoryOutcome, MappingError> {
        let summary_text = structured.get("summary").and_then(Value::as_str).ok_or(
            MappingError::MissingField {
                tool: TOOL,
                field: "summary",
            },
        )?;
        let summary = NonEmptyString::parse(summary_text, "write_memory_response.summary")
            .map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "summary",
                message: err.to_string(),
            })?;
        let accepted = structured.get("accepted").and_then(Value::as_bool).ok_or(
            MappingError::MissingField {
                tool: TOOL,
                field: "accepted",
            },
        )?;
        let dry_run = structured.get("dry_run").and_then(Value::as_bool).ok_or(
            MappingError::MissingField {
                tool: TOOL,
                field: "dry_run",
            },
        )?;
        let generated_refs = collect_generated_refs(structured)?;
        Ok(WriteMemoryOutcome::new(
            summary,
            accepted,
            dry_run,
            generated_refs,
        ))
    }
}

fn collect_generated_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(array) = structured.get("generated_refs").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        let raw = entry.as_str().ok_or(MappingError::WrongType {
            tool: TOOL,
            field: "generated_refs[]",
            expected: "string",
        })?;
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "generated_refs[]",
                message: err.to_string(),
            })?,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/write-memory.response.json");

    #[test]
    fn maps_canonical_write_memory_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = WriteMemoryResponseMapper::to_outcome(&structured).expect("maps");
        assert!(outcome.summary().as_str().starts_with("Prepared"));
        assert!(!outcome.accepted());
        assert!(outcome.dry_run());
        assert_eq!(outcome.generated_refs().len(), 2);
    }

    #[test]
    fn missing_summary_fails() {
        let value: Value = serde_json::json!({"accepted": true, "dry_run": false});
        let err = WriteMemoryResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::MissingField {
                tool: "write_memory",
                field: "summary"
            }
        ));
    }

    #[test]
    fn missing_accepted_fails() {
        let value: Value = serde_json::json!({"summary": "x", "dry_run": false});
        let err = WriteMemoryResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::MissingField {
                tool: "write_memory",
                field: "accepted"
            }
        ));
    }

    #[test]
    fn generated_refs_with_non_string_fails() {
        let value: Value = serde_json::json!({
            "summary": "x",
            "accepted": true,
            "dry_run": false,
            "generated_refs": [42],
        });
        let err = WriteMemoryResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::WrongType {
                tool: "write_memory",
                field: "generated_refs[]",
                expected: "string"
            }
        ));
    }
}
