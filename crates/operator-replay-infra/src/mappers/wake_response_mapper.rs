//! Map the structured content of a `kernel_wake` response to a typed
//! `WakeOutcome`. Captures the human-readable `summary` plus every navigable
//! ref the wake surfaced: the non-empty `evidence_ref`s from `wake.causal_spine`
//! AND the `source` of each `proof.evidence` item.
//!
//! The proof-evidence sources are load-bearing: a relation-sparse memory's
//! causal spine is dominated by structural `contains_entry` edges whose
//! `evidence_ref` is empty, so the spine alone surfaces nothing to anchor on.
//! The wake's `proof.evidence` still lists the about's salient entries (the
//! navigable refs), so surfacing those gives the operator an anchor to start
//! `kernel_near` from. Empty refs are filtered so the domain keeps `MemoryRef`
//! non-empty; the rest of the wake packet (objective, `open_loops`,
//! `next_actions`) is ignored until a use case needs it.

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
        let mut surfaced_refs = collect_causal_spine_refs(structured)?;
        for memory_ref in collect_proof_evidence_refs(structured)? {
            if !surfaced_refs.contains(&memory_ref) {
                surfaced_refs.push(memory_ref);
            }
        }
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

/// The navigable entry refs the wake surfaced as `proof.evidence` sources. These
/// are the salient about entries (e.g. `wkshop-01`) — the anchor the operator
/// needs when the causal spine carries no non-empty evidence ref.
fn collect_proof_evidence_refs(structured: &Value) -> Result<Vec<MemoryRef>, MappingError> {
    let Some(evidence) = structured
        .get("proof")
        .and_then(|proof| proof.get("evidence"))
    else {
        return Ok(Vec::new());
    };
    let array = evidence.as_array().ok_or(MappingError::WrongType {
        tool: TOOL,
        field: "proof.evidence",
        expected: "array",
    })?;
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        let Some(raw) = entry.get("source").and_then(Value::as_str) else {
            continue;
        };
        if raw.is_empty() {
            continue;
        }
        out.push(
            MemoryRef::parse(raw).map_err(|err| MappingError::InvalidValue {
                tool: TOOL,
                field: "proof.evidence[].source",
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

    #[test]
    fn surfaces_proof_evidence_sources_as_navigable_anchors() {
        // A relation-sparse memory: the causal spine is all structural edges
        // with empty evidence_ref, but proof.evidence lists the about's entries.
        let value: Value = serde_json::json!({
            "summary": "Objective: count workshops\nStatus: ACTIVE\nNext: read",
            "wake": {
                "causal_spine": [
                    {"claim": "dimension -> wkshop-01", "because": "contains", "evidence_ref": ""}
                ]
            },
            "proof": {
                "evidence": [
                    {"id": "detail:wkshop-01", "source": "wkshop-01", "supports": ["wkshop-01"], "text": "Paid product workshop #1."},
                    {"id": "detail:decoy-eu-1", "source": "decoy-eu-1", "supports": ["decoy-eu-1"], "text": "No workshop ran."}
                ]
            }
        });

        let outcome = WakeResponseMapper::to_outcome(&value).expect("proof sources surface");
        let refs: Vec<&str> = outcome
            .surfaced_refs()
            .iter()
            .map(MemoryRef::as_str)
            .collect();
        assert_eq!(outcome.surfaced_refs().len(), 2);
        assert!(refs.contains(&"wkshop-01"));
        assert!(refs.contains(&"decoy-eu-1"));
    }

    #[test]
    fn unions_spine_and_proof_refs_without_duplicates() {
        // A ref present in BOTH the causal spine and proof.evidence appears once.
        let value: Value = serde_json::json!({
            "summary": "Objective: x\nStatus: ACTIVE",
            "wake": {
                "causal_spine": [
                    {"claim": "a -> wkshop-01", "because": "causal", "evidence_ref": "wkshop-01"}
                ]
            },
            "proof": {
                "evidence": [
                    {"id": "detail:wkshop-01", "source": "wkshop-01", "supports": ["wkshop-01"], "text": "dup"},
                    {"id": "detail:wkshop-04", "source": "wkshop-04", "supports": ["wkshop-04"], "text": "new"}
                ]
            }
        });

        let outcome = WakeResponseMapper::to_outcome(&value).expect("maps");
        let refs: Vec<&str> = outcome
            .surfaced_refs()
            .iter()
            .map(MemoryRef::as_str)
            .collect();
        assert_eq!(outcome.surfaced_refs().len(), 2, "wkshop-01 not duplicated");
        assert_eq!(refs[0], "wkshop-01", "spine ref kept first");
        assert!(refs.contains(&"wkshop-04"));
    }
}
