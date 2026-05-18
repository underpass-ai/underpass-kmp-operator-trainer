use operator_shared_contract::escalate_action_dto::EscalateActionDto;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_contract::stop_action_dto::StopActionDto;
use operator_shared_contract::tool_call_action_dto::ToolCallActionDto;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;

use crate::mappers::mapping_error::MappingError;
use crate::mappers::tool_arguments_mapper::ToolArgumentsMapper;

#[derive(Debug)]
pub struct OperatorActionMapper;

impl OperatorActionMapper {
    pub fn to_domain(dto: &OperatorActionDto) -> Result<OperatorAction, MappingError> {
        match dto {
            OperatorActionDto::ToolCall(call) => {
                let arguments: ToolArguments = ToolArgumentsMapper::to_domain(&call.arguments)?;
                Ok(OperatorAction::ToolCall(ToolCallAction::new(arguments)))
            }
            OperatorActionDto::Stop(stop) => {
                let reason = StopReason::parse(&stop.reason)?;
                let mut evidence = Vec::with_capacity(stop.evidence.len());
                for raw in &stop.evidence {
                    evidence.push(MemoryRef::parse(raw.clone())?);
                }
                Ok(OperatorAction::Stop(StopAction::new(
                    reason,
                    stop.answer.clone(),
                    evidence,
                )?))
            }
            OperatorActionDto::Escalate(escalate) => {
                let reason = EscalateReason::parse(&escalate.reason)?;
                let target = ModelId::parse(escalate.target_model.clone())?;
                Ok(OperatorAction::Escalate(EscalateAction::new(
                    reason, target,
                )))
            }
        }
    }

    pub fn to_dto(domain: &OperatorAction) -> Result<OperatorActionDto, MappingError> {
        match domain {
            OperatorAction::ToolCall(call) => {
                let arguments = ToolArgumentsMapper::to_dto(call.arguments())?;
                Ok(OperatorActionDto::ToolCall(ToolCallActionDto { arguments }))
            }
            OperatorAction::Stop(stop) => Ok(OperatorActionDto::Stop(StopActionDto {
                reason: stop.reason().as_str().to_string(),
                answer: stop.answer().map(|nes| nes.as_str().to_string()),
                evidence: stop
                    .evidence()
                    .iter()
                    .map(|r| r.as_str().to_string())
                    .collect(),
            })),
            OperatorAction::Escalate(esc) => Ok(OperatorActionDto::Escalate(EscalateActionDto {
                reason: esc.reason().as_str().to_string(),
                target_model: esc.target_model().as_str().to_string(),
            })),
        }
    }
}
