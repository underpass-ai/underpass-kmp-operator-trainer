//! Mapper between calibration subject DTO and domain object.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_infra::mappers::visible_state_mapper::VisibleStateMapper;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

use crate::dto::calibration_subject_dto::CalibrationSubjectDto;
use crate::errors::calibration_case_mapping_error::CalibrationCaseMappingError;

#[derive(Debug)]
pub struct CalibrationSubjectMapper;

impl CalibrationSubjectMapper {
    pub fn to_domain(
        dto: &CalibrationSubjectDto,
    ) -> Result<CalibrationSubject, CalibrationCaseMappingError> {
        let mode = OperatorMode::parse(&dto.mode)?;
        let mut tools = Vec::with_capacity(dto.allowed_tools.len());
        for raw in &dto.allowed_tools {
            tools.push(KernelTool::parse(raw)?);
        }
        Ok(CalibrationSubject::new(
            AboutId::parse(dto.about.clone())?,
            mode,
            TaskFamily::parse(dto.task_family.clone())?,
            TrajectoryGoal::parse(dto.goal.clone())?,
            AllowedTools::parse(mode, tools)?,
            VisibleStateMapper::to_domain(&dto.visible_state)?,
        )?)
    }

    pub fn to_dto(domain: &CalibrationSubject) -> CalibrationSubjectDto {
        CalibrationSubjectDto {
            about: domain.about().as_str().to_string(),
            mode: domain.mode().as_str().to_string(),
            task_family: domain.task_family().as_str().to_string(),
            goal: domain.goal().as_str().to_string(),
            allowed_tools: domain
                .allowed_tools()
                .as_slice()
                .iter()
                .map(|tool| tool.as_str().to_string())
                .collect(),
            visible_state: VisibleStateMapper::to_dto(domain.visible_state()),
        }
    }
}
