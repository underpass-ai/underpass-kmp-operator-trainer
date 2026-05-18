//! Map the structured content of a `kernel_ask` response to a typed
//! `AskOutcome`. `answer` is absent for the UNKNOWN policy outcome;
//! `because[].ref` becomes `evidence_refs`.

use operator_shared_domain::tool_outcomes::ask_outcome::AskOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "ask";

#[derive(Debug)]
pub struct AskResponseMapper;

impl AskResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<AskOutcome, MappingError> {
        let summary = parse_required_string(structured, "summary")?;
        let answer = match structured.get("answer").and_then(Value::as_str) {
            Some(text) => Some(NonEmptyString::parse(text, "ask_response.answer").map_err(
                |err| MappingError::InvalidValue {
                    tool: TOOL,
                    field: "answer",
                    message: err.to_string(),
                },
            )?),
            None => None,
        };
        let evidence_refs = collect_because_refs(structured)?;
        Ok(AskOutcome::new(summary, answer, evidence_refs))
    }
}

fn parse_required_string(
    value: &Value,
    field: &'static str,
) -> Result<NonEmptyString, MappingError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MappingError::MissingField { tool: TOOL, field })?;
    NonEmptyString::parse(text, "ask_response.field").map_err(|err| MappingError::InvalidValue {
        tool: TOOL,
        field,
        message: err.to_string(),
    })
}

fn collect_because_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(array) = structured.get("because").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        let Some(raw) = entry.get("ref").and_then(Value::as_str) else {
            continue;
        };
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "because[].ref",
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/ask.response.json");

    #[test]
    fn maps_canonical_ask_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = AskResponseMapper::to_outcome(&structured).expect("maps");
        assert!(
            outcome
                .summary()
                .as_str()
                .starts_with("Deterministic memory answer")
        );
        assert!(outcome.answer().is_some());
        assert_eq!(
            outcome.answer().unwrap().as_str(),
            "Later she corrected it: the move is to Austin."
        );
        assert_eq!(outcome.evidence_refs().len(), 1);
        assert_eq!(outcome.evidence_refs()[0].as_str(), "claim:rachel-austin");
    }

    #[test]
    fn missing_answer_maps_to_unknown() {
        let value: Value = serde_json::json!({
            "summary": "no answer",
            "because": []
        });
        let outcome = AskResponseMapper::to_outcome(&value).unwrap();
        assert!(outcome.is_unknown());
    }

    #[test]
    fn missing_summary_fails() {
        let value: Value = serde_json::json!({"because": []});
        let err = AskResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::MissingField {
                tool: "ask",
                field: "summary"
            }
        ));
    }

    #[test]
    fn invalid_because_ref_fails() {
        let value: Value = serde_json::json!({
            "summary": "x",
            "because": [{"ref": ""}],
        });
        let err = AskResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::InvalidValue {
                tool: "ask",
                field: "because[].ref",
                ..
            }
        ));
    }
}
