use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;

use crate::mappers::mapping_error::MappingError;
use crate::mappers::operator_action_mapper::OperatorActionMapper;
use crate::mappers::visible_state_mapper::VisibleStateMapper;

#[derive(Debug)]
pub struct TrainingTrajectoryMapper;

impl TrainingTrajectoryMapper {
    pub fn to_domain(dto: &TrainingTrajectoryDto) -> Result<TrainingTrajectory, MappingError> {
        let id = TrainingTrajectoryId::parse(dto.id.clone())?;
        let step_id = StepId::parse(dto.step_id.clone())?;
        let about = AboutId::parse(dto.about.clone())?;
        let mode = OperatorMode::parse(&dto.mode)?;
        let task_family = TaskFamily::parse(dto.task_family.clone())?;
        let goal = TrajectoryGoal::parse(dto.goal.clone())?;
        let mut tools = Vec::with_capacity(dto.allowed_tools.len());
        for raw in &dto.allowed_tools {
            tools.push(KernelTool::parse(raw)?);
        }
        let allowed_tools = AllowedTools::parse(mode, tools)?;
        let visible_state = VisibleStateMapper::to_domain(&dto.visible_state)?;
        let target_action = OperatorActionMapper::to_domain(&dto.target_action)?;
        TrainingTrajectory::new(
            id,
            step_id,
            about,
            mode,
            task_family,
            goal,
            allowed_tools,
            visible_state,
            target_action,
        )
        .map_err(MappingError::Domain)
    }

    pub fn to_dto(domain: &TrainingTrajectory) -> Result<TrainingTrajectoryDto, MappingError> {
        Ok(TrainingTrajectoryDto {
            id: domain.id().as_str().to_string(),
            step_id: domain.step_id().as_str().to_string(),
            about: domain.about().as_str().to_string(),
            mode: domain.mode().as_str().to_string(),
            task_family: domain.task_family().as_str().to_string(),
            goal: domain.goal().as_str().to_string(),
            allowed_tools: domain
                .allowed_tools()
                .as_slice()
                .iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            visible_state: VisibleStateMapper::to_dto(domain.visible_state()),
            target_action: OperatorActionMapper::to_dto(domain.target_action())?,
        })
    }
}
