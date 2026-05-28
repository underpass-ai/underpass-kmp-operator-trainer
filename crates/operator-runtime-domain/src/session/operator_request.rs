use operator_shared_domain::action::operator_action::OperatorAction;
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
    // Provisional pass-through: when an upstream caller already knows the
    // operator action it wants to emit (e.g. a v8.1.5 ask query computed by a
    // future planner, or the prepared write payload reproduced by the replay
    // CLI from a corpus row), it can supply it here. The runtime does NOT
    // synthesize this field; it only propagates it into the CalibrationSubject
    // so the model can copy from it. Tool-vs-allowed-tools validation lives in
    // CalibrationSubject::new, not here.
    prepared_action: Option<OperatorAction>,
}

impl OperatorRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: OperatorSessionId,
        goal: TrajectoryGoal,
        initial_visible_state: VisibleState,
        mode: OperatorMode,
        allowed_tools: AllowedTools,
        initial_budget: SessionBudget,
        about: AboutId,
        prepared_action: Option<OperatorAction>,
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
            prepared_action,
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

    pub fn prepared_action(&self) -> Option<&OperatorAction> {
        self.prepared_action.as_ref()
    }
}
