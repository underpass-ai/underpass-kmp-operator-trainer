//! Mapper between external scenario DTOs and application scenario values.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::cursor::cursor_kind::CursorKind;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::ports::scenario_id::ScenarioId;
use operator_synthetic_domain::case::synthetic_acceptance_criteria::SyntheticAcceptanceCriteria;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;
use sha2::{Digest, Sha256};

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
            acceptance_criteria(dto)?,
            subject_hash(&dto.subject)?,
        ))
    }
}

fn acceptance_criteria(
    dto: &ScenarioDto,
) -> Result<SyntheticAcceptanceCriteria, ScenarioMappingError> {
    let Some(criteria) = &dto.acceptance_criteria else {
        return Ok(SyntheticAcceptanceCriteria::permissive());
    };
    let stop_reason = criteria
        .expected_stop_reason
        .as_deref()
        .map(StopReason::parse)
        .transpose()?;
    let cursor_kind = criteria
        .expected_cursor_kind
        .as_deref()
        .map(CursorKind::parse)
        .transpose()?;
    Ok(SyntheticAcceptanceCriteria::new(stop_reason, cursor_kind))
}

fn subject_hash(
    dto: &crate::dto::calibration_subject_dto::CalibrationSubjectDto,
) -> Result<SubjectHash, ScenarioMappingError> {
    let subject_json =
        serde_json::to_string_pretty(dto).map_err(|err| ScenarioMappingError::Serialization {
            message: err.to_string(),
        })?;
    let mut hasher = Sha256::new();
    hasher.update(subject_json.as_bytes());
    SubjectHash::parse(format!("{:x}", hasher.finalize())).map_err(ScenarioMappingError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_contract::budget_snapshot_dto::BudgetSnapshotDto;
    use operator_shared_contract::visible_state_dto::VisibleStateDto;

    use crate::dto::calibration_subject_dto::CalibrationSubjectDto;
    use crate::dto::synthetic_acceptance_criteria_dto::SyntheticAcceptanceCriteriaDto;

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

    #[test]
    fn maps_semantic_acceptance_criteria() {
        let mut dto = scenario_dto();
        dto.target = "stop".to_string();
        dto.acceptance_criteria = Some(SyntheticAcceptanceCriteriaDto {
            expected_stop_reason: Some("no_candidate".to_string()),
            expected_cursor_kind: None,
        });

        let scenario = ScenarioMapper::to_application(&dto).unwrap();

        assert_eq!(
            scenario.acceptance_criteria().expected_stop_reason(),
            Some(StopReason::NoCandidate)
        );
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
                    coverage_deviation: None,
                    candidate_abouts: vec![],
                },
                prepared_action: None,
            },
            acceptance_criteria: None,
        }
    }
}
