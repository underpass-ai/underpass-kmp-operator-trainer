//! JSON Schema used to constrain teacher responses to `OperatorActionDto`.
//! It intentionally lives in infra because it is `OpenAI` wire-format glue,
//! not domain policy.

use serde_json::{Map, Value, json};

pub fn operator_action_schema() -> Value {
    json!({
        "name": "OperatorAction",
        "strict": true,
        "schema": object(json!({
            "kind": {
                "type": "string",
                "enum": ["tool_call", "stop", "escalate"]
            },
            "tool": {
                "type": "string",
                "enum": [
                    "kernel_wake",
                    "kernel_ask",
                    "kernel_near",
                    "kernel_goto",
                    "kernel_rewind",
                    "kernel_forward",
                    "kernel_trace",
                    "kernel_inspect",
                    "kernel_ingest",
                    "kernel_write_memory",
                    "none"
                ]
            },
            "arguments": {
                "anyOf": [
                    wake_arguments(),
                    ask_arguments(),
                    near_arguments(),
                    goto_arguments(),
                    rewind_arguments(),
                    forward_arguments(),
                    trace_arguments(),
                    inspect_arguments(),
                    ingest_arguments(),
                    write_memory_arguments()
                ]
            },
            "reason": {
                "type": "string",
                "enum": [
                    "answer_ready",
                    "no_candidate",
                    "budget_exhausted",
                    "ambiguous_intent",
                    "beyond_capability",
                    "low_confidence",
                    "none"
                ]
            },
            "answer": { "type": ["string", "null"] },
            "evidence": string_array(),
            "target_model": {
                "type": "string",
                "enum": ["frontier-reasoner", "claude-opus-4-7", "none"]
            }
        })),
    })
}

fn wake_arguments() -> Value {
    object(json!({
        "about": string(),
    }))
}

fn ask_arguments() -> Value {
    object(json!({
        "query": string(),
    }))
}

fn near_arguments() -> Value {
    object(json!({
        "anchor": string(),
        "dimensions": string_array(),
        "limit": nullable_integer(),
    }))
}

fn goto_arguments() -> Value {
    object(json!({
        "cursor": cursor(),
    }))
}

fn rewind_arguments() -> Value {
    temporal_arguments()
}

fn forward_arguments() -> Value {
    temporal_arguments()
}

fn temporal_arguments() -> Value {
    object(json!({
        "cursor_key": string(),
        "cursor_anchor": string(),
        "window": integer(),
    }))
}

fn trace_arguments() -> Value {
    object(json!({
        "from": string(),
        "to": nullable_string(),
        "page": integer(),
    }))
}

fn inspect_arguments() -> Value {
    object(json!({
        "target": string(),
    }))
}

fn write_memory_arguments() -> Value {
    object(json!({
        "summary": string(),
        "body": string(),
        "related": string_array(),
    }))
}

fn ingest_arguments() -> Value {
    object(json!({
        "about": string(),
        "memory": ingest_memory(),
        "provenance": {
            "anyOf": [
                { "type": "null" },
                object(json!({
                    "source_kind": string(),
                    "source_agent": string(),
                    "observed_at": string(),
                    "correlation_id": nullable_string(),
                    "causation_id": nullable_string(),
                }))
            ]
        },
        "idempotency_key": string(),
        "dry_run": nullable_boolean(),
    }))
}

fn cursor() -> Value {
    json!({
        "anyOf": [
            cursor_ref(),
            cursor_around(),
            cursor_temporal(),
            cursor_trace()
        ]
    })
}

fn cursor_ref() -> Value {
    object(json!({
        "kind": { "type": "string", "enum": ["ref"] },
        "target": string(),
    }))
}

fn cursor_around() -> Value {
    object(json!({
        "kind": { "type": "string", "enum": ["around"] },
        "anchor": string(),
        "dimensions": string_array(),
    }))
}

fn cursor_temporal() -> Value {
    object(json!({
        "kind": { "type": "string", "enum": ["temporal"] },
        "key": string(),
        "anchor": string(),
    }))
}

fn cursor_trace() -> Value {
    object(json!({
        "kind": { "type": "string", "enum": ["trace"] },
        "from": string(),
        "to": string(),
    }))
}

fn ingest_memory() -> Value {
    object(json!({
        "dimensions": {
            "type": "array",
            "items": ingest_dimension(),
        },
        "entries": {
            "type": "array",
            "items": ingest_entry(),
        },
        "relations": {
            "type": "array",
            "items": ingest_relation(),
        },
        "evidence": {
            "type": "array",
            "items": ingest_evidence(),
        },
    }))
}

fn ingest_dimension() -> Value {
    object(json!({
        "id": string(),
        "kind": string(),
        "title": nullable_string(),
        "metadata": metadata(),
    }))
}

fn ingest_entry() -> Value {
    object(json!({
        "id": string(),
        "kind": string(),
        "text": string(),
        "coordinates": {
            "type": "array",
            "items": ingest_temporal_coordinate(),
        },
        "metadata": metadata(),
    }))
}

fn ingest_temporal_coordinate() -> Value {
    object(json!({
        "dimension": string(),
        "scope_id": string(),
        "occurred_at": nullable_string(),
        "observed_at": nullable_string(),
        "ingested_at": nullable_string(),
        "valid_from": nullable_string(),
        "valid_until": nullable_string(),
        "sequence": nullable_integer(),
        "rank": nullable_integer(),
        "metadata": metadata(),
    }))
}

fn ingest_relation() -> Value {
    object(json!({
        "from": string(),
        "to": string(),
        "rel": string(),
        "class": string(),
        "why": nullable_string(),
        "evidence": nullable_string(),
        "confidence": nullable_string(),
        "sequence": nullable_integer(),
    }))
}

fn ingest_evidence() -> Value {
    object(json!({
        "id": string(),
        "supports": string_array(),
        "text": string(),
        "source": nullable_string(),
        "time": nullable_string(),
        "metadata": metadata(),
    }))
}

fn metadata() -> Value {
    json!({
        "anyOf": [
            object(json!({})),
            object(json!({ "kind": nullable_string() })),
            object(json!({ "phase": nullable_string() })),
            object(json!({ "role": nullable_string() })),
            object(json!({ "source": nullable_string() })),
            object(json!({ "template": nullable_string() })),
        ]
    })
}

fn object(properties: Value) -> Value {
    let mut required = Vec::new();
    if let Some(map) = properties.as_object() {
        required.extend(map.keys().cloned().map(Value::String));
    }
    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("additionalProperties".to_string(), Value::Bool(false));
    schema.insert("properties".to_string(), properties);
    schema.insert("required".to_string(), Value::Array(required));
    Value::Object(schema)
}

fn string() -> Value {
    json!({ "type": "string" })
}

fn nullable_string() -> Value {
    json!({ "type": ["string", "null"] })
}

fn integer() -> Value {
    json!({ "type": "integer" })
}

fn nullable_integer() -> Value {
    json!({ "type": ["integer", "null"] })
}

fn nullable_boolean() -> Value {
    json!({ "type": ["boolean", "null"] })
}

fn string_array() -> Value {
    json!({ "type": "array", "items": string() })
}

#[cfg(test)]
mod tests {
    use operator_shared_contract::operator_action_dto::OperatorActionDto;
    use serde_json::json;

    use super::operator_action_schema;

    #[test]
    fn schema_allows_operator_action_kind_not_raw_tool_kind() {
        let schema = operator_action_schema();
        let kind_enum = schema["schema"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("kind enum exists");

        assert!(kind_enum.contains(&json!("tool_call")));
        assert!(!kind_enum.contains(&json!("kernel_inspect")));
    }

    #[test]
    fn schema_exposes_kernel_inspect_argument_branch() {
        let schema = operator_action_schema();
        let branches = schema["schema"]["properties"]["arguments"]["anyOf"]
            .as_array()
            .expect("arguments anyOf exists");
        let inspect = branches
            .iter()
            .find(|branch| branch["properties"].get("target").is_some())
            .expect("kernel_inspect branch exists");

        assert_eq!(inspect["properties"]["target"], json!({ "type": "string" }));
    }

    #[test]
    fn sample_kernel_inspect_action_deserializes_as_operator_action() {
        let sample = json!({
            "kind": "tool_call",
            "tool": "kernel_inspect",
            "arguments": {
                "target": "about:id:node:X"
            },
            "reason": "none",
            "answer": null,
            "evidence": [],
            "target_model": "none"
        });

        serde_json::from_value::<OperatorActionDto>(sample).expect("sample action parses");
    }

    #[test]
    fn raw_tool_kind_does_not_deserialize_as_operator_action() {
        let raw_tool_kind = json!({
            "kind": "kernel_inspect",
            "arguments": {
                "target": "about:id:node:X"
            }
        });

        serde_json::from_value::<OperatorActionDto>(raw_tool_kind)
            .expect_err("raw tool kind must not parse as OperatorActionDto");
    }
}
