//! Map the structured content of a `kernel_inspect` response to a
//! typed `InspectOutcome`. First-pass capture: `summary`,
//! `object.{ref,kind}`, `links.incoming[].from` and
//! `links.outgoing[].to`. Inline evidence and `raw` are dropped.

use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "inspect";

#[derive(Debug)]
pub struct InspectResponseMapper;

impl InspectResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<InspectOutcome, MappingError> {
        let summary = required_string(structured, "summary")?;
        let object = structured.get("object").ok_or(MappingError::MissingField {
            tool: TOOL,
            field: "object",
        })?;
        let target_raw =
            object
                .get("ref")
                .and_then(Value::as_str)
                .ok_or(MappingError::MissingField {
                    tool: TOOL,
                    field: "object.ref",
                })?;
        let target = MemoryRef::parse(target_raw).map_err(|err| MappingError::InvalidValue {
            tool: TOOL,
            field: "object.ref",
            message: err.to_string(),
        })?;
        let kind_raw =
            object
                .get("kind")
                .and_then(Value::as_str)
                .ok_or(MappingError::MissingField {
                    tool: TOOL,
                    field: "object.kind",
                })?;
        let kind = NonEmptyString::parse(kind_raw, "inspect_response.kind").map_err(|err| {
            MappingError::InvalidValue {
                tool: TOOL,
                field: "object.kind",
                message: err.to_string(),
            }
        })?;
        let incoming_refs = collect_link_refs(structured, "incoming", "from")?;
        let outgoing_refs = collect_link_refs(structured, "outgoing", "to")?;
        Ok(InspectOutcome::new(
            summary,
            target,
            kind,
            incoming_refs,
            outgoing_refs,
        ))
    }
}

fn required_string(value: &Value, field: &'static str) -> Result<NonEmptyString, MappingError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MappingError::MissingField { tool: TOOL, field })?;
    NonEmptyString::parse(text, "inspect_response.field").map_err(|err| {
        MappingError::InvalidValue {
            tool: TOOL,
            field,
            message: err.to_string(),
        }
    })
}

fn collect_link_refs(
    structured: &Value,
    direction: &'static str,
    edge_endpoint: &'static str,
) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(links) = structured.get("links") else {
        return Ok(Vec::new());
    };
    let Some(array) = links.get(direction).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(array.len());
    for edge in array {
        let Some(raw) = edge.get(edge_endpoint).and_then(Value::as_str) else {
            continue;
        };
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "links.<direction>[].<endpoint>",
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/inspect.response.json");

    #[test]
    fn maps_canonical_inspect_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = InspectResponseMapper::to_outcome(&structured).expect("maps");
        assert!(outcome.summary().as_str().starts_with("Found"));
        assert_eq!(outcome.target().as_str(), "claim:rachel-austin");
        assert_eq!(outcome.kind().as_str(), "claim");
        assert_eq!(outcome.outgoing_refs().len(), 1);
        assert_eq!(outcome.outgoing_refs()[0].as_str(), "claim:rachel-denver");
        assert!(outcome.incoming_refs().is_empty());
    }
}
