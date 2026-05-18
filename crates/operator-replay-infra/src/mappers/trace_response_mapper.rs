//! Map the structured content of a `kernel_trace` response to a typed
//! `TraceOutcome`. The wire shape is a list of `{from, to, rel, ...}`
//! edges; this first-pass capture linearises them into
//! `[edges[0].from, edges[0].to, edges[1].to, ..., edges[n].to]`. If
//! the edges are not contiguous, the result is still well-formed but
//! does not enforce path continuity.

use operator_shared_domain::tool_outcomes::trace_outcome::TraceOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "trace";

#[derive(Debug)]
pub struct TraceResponseMapper;

impl TraceResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<TraceOutcome, MappingError> {
        let summary = required_summary(structured)?;
        let path_refs = collect_path_refs(structured)?;
        Ok(TraceOutcome::new(summary, path_refs))
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
    NonEmptyString::parse(text, "trace_response.summary").map_err(|err| {
        MappingError::InvalidValue {
            tool: TOOL,
            field: "summary",
            message: err.to_string(),
        }
    })
}

fn collect_path_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(array) = structured.get("trace").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(array.len() + 1);
    for (index, edge) in array.iter().enumerate() {
        if index == 0
            && let Some(from) = edge.get("from").and_then(Value::as_str)
        {
            out.push(
                MemoryRef::parse(from).map_err(|err| MappingError::InvalidValue {
                    tool: TOOL,
                    field: "trace[0].from",
                    message: err.to_string(),
                })?,
            );
        }
        let to = edge
            .get("to")
            .and_then(Value::as_str)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "trace[].to",
            })?;
        out.push(
            MemoryRef::parse(to).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "trace[].to",
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/trace.response.json");

    #[test]
    fn maps_canonical_trace_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = TraceResponseMapper::to_outcome(&structured).expect("maps");
        assert!(outcome.summary().as_str().contains("supersedes"));
        assert_eq!(outcome.path_refs().len(), 2);
        assert_eq!(outcome.path_refs()[0].as_str(), "claim:rachel-austin");
        assert_eq!(outcome.path_refs()[1].as_str(), "claim:rachel-denver");
    }
}
