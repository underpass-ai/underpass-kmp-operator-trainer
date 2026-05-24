use operator_shared_contract::escalate_action_dto::EscalateActionDto;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_contract::stop_action_dto::StopActionDto;
use operator_shared_contract::tool_arguments_dto::ToolArgumentsDto;
use operator_shared_contract::tool_call_action_dto::ToolCallActionDto;

use crate::adapters::openai::openai_action_inner_wire_dto::OpenAIActionInnerWireDto;
use crate::adapters::openai::openai_action_wire_dto::OpenAIActionWireDto;
use crate::adapters::openai::openai_action_wire_mapping_error::OpenAIActionWireMappingError;
use crate::adapters::openai::openai_arguments_wire_dto::OpenAIArgumentsWireDto;

#[derive(Debug)]
pub struct OpenAIActionWireDtoMapper;

impl OpenAIActionWireDtoMapper {
    pub fn to_operator_action_dto(
        wire: OpenAIActionWireDto,
    ) -> Result<OperatorActionDto, OpenAIActionWireMappingError> {
        let inner = wire.action;
        match inner.kind.as_str() {
            "tool_call" => Self::tool_call(inner),
            "stop" => Self::stop(inner),
            "escalate" => Self::escalate(inner),
            other => Err(OpenAIActionWireMappingError::new(format!(
                "unknown action kind: {other}"
            ))),
        }
    }

    #[must_use]
    pub fn from_operator_action_dto(action: &OperatorActionDto) -> OpenAIActionWireDto {
        let inner = match action {
            OperatorActionDto::ToolCall(call) => OpenAIActionInnerWireDto {
                kind: "tool_call".to_string(),
                tool: Some(call.arguments.tool.clone()),
                arguments: Some(OpenAIArgumentsWireDto::new(
                    call.arguments.arguments.clone(),
                )),
                reason: None,
                answer: None,
                evidence: Vec::new(),
                target_model: None,
            },
            OperatorActionDto::Stop(stop) => OpenAIActionInnerWireDto {
                kind: "stop".to_string(),
                tool: None,
                arguments: None,
                reason: Some(stop.reason.clone()),
                answer: stop.answer.clone(),
                evidence: stop.evidence.clone(),
                target_model: None,
            },
            OperatorActionDto::Escalate(escalate) => OpenAIActionInnerWireDto {
                kind: "escalate".to_string(),
                tool: None,
                arguments: None,
                reason: Some(escalate.reason.clone()),
                answer: None,
                evidence: Vec::new(),
                target_model: Some(escalate.target_model.clone()),
            },
        };
        OpenAIActionWireDto { action: inner }
    }

    fn tool_call(
        inner: OpenAIActionInnerWireDto,
    ) -> Result<OperatorActionDto, OpenAIActionWireMappingError> {
        let tool = require(inner.tool, "tool_call requires tool")?;
        let arguments = require(inner.arguments, "tool_call requires arguments")?;
        reject_present(inner.reason.as_ref(), "tool_call cannot carry reason")?;
        reject_present(inner.answer.as_ref(), "tool_call cannot carry answer")?;
        reject_present(
            inner.target_model.as_ref(),
            "tool_call cannot carry target_model",
        )?;
        reject_non_empty(&inner.evidence, "tool_call cannot carry evidence")?;
        Ok(OperatorActionDto::ToolCall(ToolCallActionDto {
            arguments: ToolArgumentsDto {
                tool,
                arguments: arguments.into_value(),
            },
        }))
    }

    fn stop(
        inner: OpenAIActionInnerWireDto,
    ) -> Result<OperatorActionDto, OpenAIActionWireMappingError> {
        reject_present(inner.tool.as_ref(), "stop cannot carry tool")?;
        reject_present(inner.arguments.as_ref(), "stop cannot carry arguments")?;
        let reason = require(inner.reason, "stop requires reason")?;
        reject_present(
            inner.target_model.as_ref(),
            "stop cannot carry target_model",
        )?;
        Ok(OperatorActionDto::Stop(StopActionDto {
            reason,
            answer: inner.answer,
            evidence: inner.evidence,
        }))
    }

    fn escalate(
        inner: OpenAIActionInnerWireDto,
    ) -> Result<OperatorActionDto, OpenAIActionWireMappingError> {
        reject_present(inner.tool.as_ref(), "escalate cannot carry tool")?;
        reject_present(inner.arguments.as_ref(), "escalate cannot carry arguments")?;
        let reason = require(inner.reason, "escalate requires reason")?;
        reject_present(inner.answer.as_ref(), "escalate cannot carry answer")?;
        reject_non_empty(&inner.evidence, "escalate cannot carry evidence")?;
        let target_model = require(inner.target_model, "escalate requires target_model")?;
        Ok(OperatorActionDto::Escalate(EscalateActionDto {
            reason,
            target_model,
        }))
    }
}

fn require<T>(value: Option<T>, message: &'static str) -> Result<T, OpenAIActionWireMappingError> {
    value.ok_or_else(|| OpenAIActionWireMappingError::new(message))
}

fn reject_present<T>(
    value: Option<&T>,
    message: &'static str,
) -> Result<(), OpenAIActionWireMappingError> {
    if value.is_some() {
        return Err(OpenAIActionWireMappingError::new(message));
    }
    Ok(())
}

fn reject_non_empty(
    value: &[String],
    message: &'static str,
) -> Result<(), OpenAIActionWireMappingError> {
    if value.is_empty() {
        return Ok(());
    }
    Err(OpenAIActionWireMappingError::new(message))
}
