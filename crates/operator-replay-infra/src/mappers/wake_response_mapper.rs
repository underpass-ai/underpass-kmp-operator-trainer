//! Map the structured content of a `kernel_wake` response to a typed
//! `WakeOutcome`. First-pass capture: the human-readable `summary`
//! plus every non-empty `evidence_ref` collected from `wake.causal_spine`.
//! Live kernel responses can include structural causal-spine entries with an
//! empty evidence ref; those are filtered at the infra boundary so the domain
//! keeps `MemoryRef` non-empty. The richer wake packet (objective,
//! `open_loops`, `next_actions`, proof) is ignored until a use case needs it.

use operator_shared_domain::tool_outcomes::wake_outcome::WakeOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "wake";

#[derive(Debug)]
pub struct WakeResponseMapper;

impl WakeResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<WakeOutcome, MappingError> {
        let summary_text = structured.get("summary").and_then(Value::as_str).ok_or(
            MappingError::MissingField {
                tool: TOOL,
                field: "summary",
            },
        )?;
        let summary =
            NonEmptyString::parse(summary_text, "wake_response.summary").map_err(|err| {
                MappingError::InvalidValue {
                    tool: TOOL,
                    field: "summary",
                    message: err.to_string(),
                }
            })?;
        let surfaced_refs = collect_causal_spine_refs(structured)?;
        Ok(WakeOutcome::new(summary, surfaced_refs))
    }
}

fn collect_causal_spine_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(wake) = structured.get("wake") else {
        return Ok(Vec::new());
    };
    let Some(causal_spine) = wake.get("causal_spine") else {
        return Ok(Vec::new());
    };
    let array = causal_spine.as_array().ok_or(MappingError::WrongType {
        tool: TOOL,
        field: "wake.causal_spine",
        expected: "array",
    })?;
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        let Some(raw) = entry.get("evidence_ref").and_then(Value::as_str) else {
            continue;
        };
        if raw.is_empty() {
            continue;
        }
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "wake.causal_spine[].evidence_ref",
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
        include_str!("../../../../api/mcp/examples/kernel/v1beta1/kmp/wake.response.json");

    #[test]
    fn maps_canonical_wake_fixture() {
        let structured: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let outcome = WakeResponseMapper::to_outcome(&structured).expect("maps");
        assert!(
            outcome
                .summary()
                .as_str()
                .starts_with("Continue with Kernel Memory Protocol")
        );
        assert_eq!(outcome.surfaced_refs().len(), 1);
        assert_eq!(
            outcome.surfaced_refs()[0].as_str(),
            "evidence:kmp-design-direction"
        );
    }

    #[test]
    fn missing_summary_fails() {
        let value: Value = serde_json::json!({"wake": {"causal_spine": []}});
        let err = WakeResponseMapper::to_outcome(&value).unwrap_err();
        assert!(matches!(
            err,
            MappingError::MissingField {
                tool: "wake",
                field: "summary"
            }
        ));
    }

    #[test]
    fn filters_empty_causal_spine_evidence_refs() {
        let value: Value = serde_json::json!({
            "summary": "Objective: live anchor\nStatus: ACTIVE\nNext: continue",
            "wake": {
                "causal_spine": [
                    {
                        "claim": "anchor -> dimension",
                        "because": "Memory anchor includes this dimension.",
                        "evidence_ref": ""
                    },
                    {
                        "claim": "evidence -> claim",
                        "because": "Evidence supports this memory entry.",
                        "evidence_ref": "article:20260504T233722Z:evidence:frontend-cache"
                    },
                    {
                        "claim": "anchor -> claim",
                        "because": "Memory anchor records this entry.",
                        "evidence_ref": ""
                    }
                ]
            }
        });

        let outcome = WakeResponseMapper::to_outcome(&value).expect("empty refs are filtered");

        assert_eq!(outcome.surfaced_refs().len(), 1);
        assert_eq!(
            outcome.surfaced_refs()[0].as_str(),
            "article:20260504T233722Z:evidence:frontend-cache"
        );
    }
}
