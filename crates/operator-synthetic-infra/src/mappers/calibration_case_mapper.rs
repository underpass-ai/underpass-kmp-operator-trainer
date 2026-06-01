//! Mapper between calibration case DTO and domain aggregate.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_domain::calibration::accepted_actions::AcceptedActions;
use operator_synthetic_domain::calibration::calibration_case::CalibrationCase;
use operator_synthetic_domain::calibration::calibration_case_category::CalibrationCaseCategory;
use operator_synthetic_domain::calibration::calibration_case_id::CalibrationCaseId;
use operator_synthetic_domain::calibration::calibration_domain_theme::CalibrationDomainTheme;
use operator_synthetic_domain::calibration::expected_action_rationale::ExpectedActionRationale;

use crate::dto::calibration_case_dto::CalibrationCaseDto;
use crate::errors::calibration_case_mapping_error::CalibrationCaseMappingError;
use crate::mappers::calibration_subject_mapper::CalibrationSubjectMapper;

#[derive(Debug)]
pub struct CalibrationCaseMapper;

impl CalibrationCaseMapper {
    pub fn to_domain(
        dto: &CalibrationCaseDto,
    ) -> Result<CalibrationCase, CalibrationCaseMappingError> {
        let mut accepted_actions: Vec<OperatorAction> =
            Vec::with_capacity(dto.accepted_actions.len());
        for action_dto in &dto.accepted_actions {
            accepted_actions.push(OperatorActionMapper::to_domain(action_dto)?);
        }
        Ok(CalibrationCase::new(
            CalibrationCaseId::parse(dto.case_id.clone())?,
            CalibrationDomainTheme::parse(&dto.domain_theme)?,
            CalibrationCaseCategory::parse(&dto.category)?,
            CalibrationSubjectMapper::to_domain(&dto.subject)?,
            AcceptedActions::new(accepted_actions)?,
            ExpectedActionRationale::parse(dto.expected_action_rationale.clone())?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_contract::budget_snapshot_dto::BudgetSnapshotDto;
    use operator_shared_contract::operator_action_dto::OperatorActionDto;
    use operator_shared_contract::stop_action_dto::StopActionDto;
    use operator_shared_contract::tool_arguments_dto::ToolArgumentsDto;
    use operator_shared_contract::tool_call_action_dto::ToolCallActionDto;
    use operator_shared_contract::visible_state_dto::VisibleStateDto;
    use serde_json::json;

    use crate::dto::calibration_subject_dto::CalibrationSubjectDto;

    #[test]
    fn parses_minimal_case() {
        let case = CalibrationCaseMapper::to_domain(&case_dto(vec![inspect_action()])).unwrap();
        assert_eq!(case.case_id().as_str(), "calib:inspect");
    }

    #[test]
    fn parses_case_with_multiple_accepted_actions() {
        let case =
            CalibrationCaseMapper::to_domain(&case_dto(vec![inspect_action(), inspect_action()]))
                .unwrap();
        assert_eq!(case.accepted_actions().len(), 2);
    }

    #[test]
    fn rejects_case_with_empty_accepted_actions() {
        assert!(CalibrationCaseMapper::to_domain(&case_dto(vec![])).is_err());
    }

    #[test]
    fn parses_subject_with_prepared_tool_call() {
        let mut dto = case_dto(vec![inspect_action()]);
        dto.subject.prepared_action = Some(inspect_action());
        let case = CalibrationCaseMapper::to_domain(&dto).unwrap();
        let prepared = case.subject().prepared_action().unwrap();
        assert_eq!(prepared.action(), &case.accepted_actions().as_slice()[0]);
    }

    #[test]
    fn rejects_subject_with_prepared_stop() {
        let mut dto = case_dto(vec![inspect_action()]);
        dto.subject.prepared_action = Some(OperatorActionDto::Stop(StopActionDto {
            reason: "answer_ready".to_string(),
            answer: None,
            evidence: vec![],
        }));
        assert!(CalibrationCaseMapper::to_domain(&dto).is_err());
    }

    fn case_dto(accepted_actions: Vec<OperatorActionDto>) -> CalibrationCaseDto {
        CalibrationCaseDto {
            case_id: "calib:inspect".to_string(),
            domain_theme: "technical_incident".to_string(),
            category: "happy".to_string(),
            subject: CalibrationSubjectDto {
                about: "about:incident".to_string(),
                mode: "read".to_string(),
                task_family: "read.inspect".to_string(),
                goal: "Inspect visible evidence.".to_string(),
                allowed_tools: vec![
                    "kernel_wake".to_string(),
                    "kernel_ask".to_string(),
                    "kernel_near".to_string(),
                    "kernel_goto".to_string(),
                    "kernel_rewind".to_string(),
                    "kernel_forward".to_string(),
                    "kernel_trace".to_string(),
                    "kernel_inspect".to_string(),
                ],
                visible_state: VisibleStateDto {
                    known_refs: vec!["node:target".to_string()],
                    known_dimensions: vec![],
                    active_cursor: None,
                    budget: BudgetSnapshotDto {
                        calls_remaining: Some(3),
                        tokens_remaining: Some(1024),
                    },
                    coverage_deviation: None,
                    candidate_abouts: vec![],
                },
                prepared_action: None,
            },
            accepted_actions,
            expected_action_rationale: "The ref is already visible.".to_string(),
        }
    }

    fn inspect_action() -> OperatorActionDto {
        OperatorActionDto::ToolCall(ToolCallActionDto {
            arguments: ToolArgumentsDto {
                tool: "kernel_inspect".to_string(),
                arguments: json!({ "target": "node:target" }),
            },
        })
    }
}
