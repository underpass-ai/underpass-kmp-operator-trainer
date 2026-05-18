//! Map the structured content of a `kernel_rewind` response to a typed
//! `RewindOutcome`. Same wire shape as `kernel_near` for this first-pass
//! capture.

use operator_shared_domain::tool_outcomes::rewind_outcome::RewindOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "rewind";

#[derive(Debug)]
pub struct RewindResponseMapper;

impl RewindResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<RewindOutcome, MappingError> {
        let summary = required_summary(structured)?;
        let entry_refs = collect_entry_refs(structured)?;
        Ok(RewindOutcome::new(summary, entry_refs))
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
    NonEmptyString::parse(text, "rewind_response.summary").map_err(|err| {
        MappingError::InvalidValue {
            tool: TOOL,
            field: "summary",
            message: err.to_string(),
        }
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/rewind.response.json");

    #[test]
    fn maps_canonical_rewind_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = RewindResponseMapper::to_outcome(&structured).expect("maps");
        assert!(outcome.summary().as_str().contains("Denver"));
        assert_eq!(outcome.entry_refs().len(), 1);
    }
}
