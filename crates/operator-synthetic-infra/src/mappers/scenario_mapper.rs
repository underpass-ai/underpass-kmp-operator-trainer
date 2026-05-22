//! Mapper between external scenario DTOs and application scenario values.

use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::ports::scenario_id::ScenarioId;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::dto::scenario_dto::ScenarioDto;
use crate::errors::scenario_mapping_error::ScenarioMappingError;
use crate::mappers::calibration_subject_mapper::CalibrationSubjectMapper;

#[derive(Debug)]
pub struct ScenarioMapper;

impl ScenarioMapper {
    pub fn to_application(dto: &ScenarioDto) -> Result<Scenario, ScenarioMappingError> {
        Ok(Scenario::new(
            ScenarioId::parse(dto.scenario_id.clone())?,
            SyntheticGenerationTarget::parse(&dto.target)?,
            CalibrationSubjectMapper::to_domain(&dto.subject)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_contract::budget_snapshot_dto::BudgetSnapshotDto;
    use operator_shared_contract::visible_state_dto::VisibleStateDto;

    use crate::dto::calibration_subject_dto::CalibrationSubjectDto;

    #[test]
    fn maps_valid_scenario() {
        let scenario = ScenarioMapper::to_application(&scenario_dto()).unwrap();
        assert_eq!(scenario.id().as_str(), "scenario:inspect");
        assert_eq!(scenario.target().name(), "inspect");
    }

    #[test]
    fn rejects_unknown_target() {
        let mut dto = scenario_dto();
        dto.target = "kernel_unknown".to_string();
        assert!(ScenarioMapper::to_application(&dto).is_err());
    }

    fn scenario_dto() -> ScenarioDto {
        ScenarioDto {
            scenario_id: "scenario:inspect".to_string(),
            target: "kernel_inspect".to_string(),
            subject: CalibrationSubjectDto {
                about: "about:incident".to_string(),
                mode: "read".to_string(),
                task_family: "realistic.inspect".to_string(),
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
                },
                prepared_action: None,
            },
        }
    }
}
