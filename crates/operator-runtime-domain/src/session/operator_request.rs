use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::budget::session_budget::SessionBudget;
use crate::error::runtime_domain_error::RuntimeDomainError;
use crate::error::runtime_domain_result::RuntimeDomainResult;
use crate::session::operator_session_id::OperatorSessionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRequest {
    session_id: OperatorSessionId,
    goal: TrajectoryGoal,
    initial_visible_state: VisibleState,
    mode: OperatorMode,
    allowed_tools: AllowedTools,
    initial_budget: SessionBudget,
    about: AboutId,
}

impl OperatorRequest {
    pub fn new(
        session_id: OperatorSessionId,
        goal: TrajectoryGoal,
        initial_visible_state: VisibleState,
        mode: OperatorMode,
        allowed_tools: AllowedTools,
        initial_budget: SessionBudget,
        about: AboutId,
    ) -> RuntimeDomainResult<Self> {
        Self::validate_allowed_tools(mode, &allowed_tools)?;
        Ok(Self {
            session_id,
            goal,
            initial_visible_state,
            mode,
            allowed_tools,
            initial_budget,
            about,
        })
    }

    fn validate_allowed_tools(
        mode: OperatorMode,
        allowed_tools: &AllowedTools,
    ) -> RuntimeDomainResult<()> {
        if allowed_tools.mode() != mode {
            return Err(RuntimeDomainError::AllowedToolsModeMismatch {
                expected: mode.as_str(),
                actual: allowed_tools.mode().as_str(),
            });
        }
        if !matches!(mode, OperatorMode::Write | OperatorMode::Full) {
            for tool in [KernelTool::Ingest, KernelTool::WriteMemory] {
                if allowed_tools.contains(tool) {
                    return Err(RuntimeDomainError::ToolNotAllowedInRuntimeMode {
                        mode: mode.as_str(),
                        tool,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn session_id(&self) -> &OperatorSessionId {
        &self.session_id
    }

    pub fn goal(&self) -> &TrajectoryGoal {
        &self.goal
    }

    pub fn initial_visible_state(&self) -> &VisibleState {
        &self.initial_visible_state
    }

    pub fn mode(&self) -> OperatorMode {
        self.mode
    }

    pub fn allowed_tools(&self) -> &AllowedTools {
        &self.allowed_tools
    }

    pub fn initial_budget(&self) -> SessionBudget {
        self.initial_budget
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }
}
