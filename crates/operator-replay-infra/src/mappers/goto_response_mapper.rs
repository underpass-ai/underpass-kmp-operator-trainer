//! Map the structured content of a `kernel_goto` response to a typed
//! `GotoOutcome`. Same wire shape as `kernel_near` for this first-pass
//! capture.

use operator_shared_domain::tool_outcomes::goto_outcome::GotoOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "goto";

#[derive(Debug)]
pub struct GotoResponseMapper;

impl GotoResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<GotoOutcome, MappingError> {
        let summary = required_summary(structured)?;
        let entry_refs = collect_entry_refs(structured)?;
        Ok(GotoOutcome::new(summary, entry_refs))
    }
}

fn required_summary(structured: &Value) -> Result<NonEmptyString, MappingError> {
    let text =
        structured
            .get("summary")
            .and_then(Value::as_str)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "summary",
            })?;
    NonEmptyString::parse(text, "goto_response.summary").map_err(|err| MappingError::InvalidValue {
        tool: TOOL,
        field: "summary",
        message: err.to_string(),
    })
}

fn collect_entry_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(array) = structured.get("entries").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        let raw = entry
            .get("ref")
            .and_then(Value::as_str)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "entries[].ref",
            })?;
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "entries[].ref",
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/goto.response.json");

    #[test]
    fn maps_canonical_goto_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = GotoResponseMapper::to_outcome(&structured).expect("maps");
        assert!(outcome.summary().as_str().contains("Denver"));
        assert_eq!(outcome.entry_refs().len(), 1);
        assert_eq!(outcome.entry_refs()[0].as_str(), "claim:rachel-denver");
    }

    #[test]
    fn missing_summary_fails() {
        let value: Value = serde_json::json!({"entries": []});
        let err = GotoResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::MissingField {
                tool: "goto",
                field: "summary"
            }
        ));
    }
}
