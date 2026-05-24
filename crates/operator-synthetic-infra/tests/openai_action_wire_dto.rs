use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_synthetic_infra::adapters::openai::openai_action_wire_dto::OpenAIActionWireDto;
use operator_synthetic_infra::adapters::openai::openai_action_wire_dto_mapper::OpenAIActionWireDtoMapper;
use serde_json::json;

#[test]
fn valid_stop_round_trips_through_operator_action_dto() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "stop",
            "tool": null,
            "arguments": null,
            "reason": "answer_ready",
            "answer": "Evidence is sufficient.",
            "evidence": ["about:test:node:evidence"],
            "target_model": null
        }
    }))
    .expect("wire parses");

    let action =
        OpenAIActionWireDtoMapper::to_operator_action_dto(wire.clone()).expect("stop wire maps");
    assert!(matches!(action, OperatorActionDto::Stop(_)));
    assert_eq!(
        OpenAIActionWireDtoMapper::from_operator_action_dto(&action),
        wire
    );
}

#[test]
fn valid_tool_call_round_trips_through_operator_action_dto() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "tool_call",
            "tool": "kernel_inspect",
            "arguments": {"target": "about:test:node:evidence"},
            "reason": null,
            "answer": null,
            "evidence": [],
            "target_model": null
        }
    }))
    .expect("wire parses");

    let action = OpenAIActionWireDtoMapper::to_operator_action_dto(wire.clone())
        .expect("tool_call wire maps");
    assert!(matches!(action, OperatorActionDto::ToolCall(_)));
    assert_eq!(
        OpenAIActionWireDtoMapper::from_operator_action_dto(&action),
        wire
    );
}

#[test]
fn valid_escalate_round_trips_through_operator_action_dto() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "escalate",
            "tool": null,
            "arguments": null,
            "reason": "beyond_capability",
            "answer": null,
            "evidence": [],
            "target_model": "frontier-reasoner"
        }
    }))
    .expect("wire parses");

    let action = OpenAIActionWireDtoMapper::to_operator_action_dto(wire.clone())
        .expect("escalate wire maps");
    assert!(matches!(action, OperatorActionDto::Escalate(_)));
    assert_eq!(
        OpenAIActionWireDtoMapper::from_operator_action_dto(&action),
        wire
    );
}

#[test]
fn stop_with_tool_is_shape_violation() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "stop",
            "tool": "kernel_inspect",
            "arguments": null,
            "reason": "answer_ready",
            "answer": "Evidence is sufficient.",
            "evidence": [],
            "target_model": null
        }
    }))
    .expect("wire parses");

    let err = OpenAIActionWireDtoMapper::to_operator_action_dto(wire).expect_err("shape fails");
    assert_eq!(err.to_string(), "stop cannot carry tool");
}

#[test]
fn tool_call_without_tool_is_shape_violation() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "tool_call",
            "tool": null,
            "arguments": {"target": "about:test:node:evidence"},
            "reason": null,
            "answer": null,
            "evidence": [],
            "target_model": null
        }
    }))
    .expect("wire parses");

    let err = OpenAIActionWireDtoMapper::to_operator_action_dto(wire).expect_err("shape fails");
    assert_eq!(err.to_string(), "tool_call requires tool");
}

#[test]
fn escalate_without_target_model_is_shape_violation() {
    let wire: OpenAIActionWireDto = serde_json::from_value(json!({
        "action": {
            "kind": "escalate",
            "tool": null,
            "arguments": null,
            "reason": "beyond_capability",
            "answer": null,
            "evidence": [],
            "target_model": null
        }
    }))
    .expect("wire parses");

    let err = OpenAIActionWireDtoMapper::to_operator_action_dto(wire).expect_err("shape fails");
    assert_eq!(err.to_string(), "escalate requires target_model");
}
