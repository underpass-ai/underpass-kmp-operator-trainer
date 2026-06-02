//! Compile a window-expansion episode into a runtime [`OperatorRequest`].
//!
//! This is the synthetic-to-runtime bridge for the generator: it turns a
//! [`WindowExpansionSpec`] (the bounded wake window size and how many expansion
//! steps are allowed) plus a session's `about` and `goal` into the request the
//! multi-step loop drives. It encodes the fixed envelope of a window-expansion
//! session — it is a **read** session over the canonical read tools, and its
//! call budget is sized to the spec so the policy gets exactly `max_iterations`
//! expansion calls plus one terminal decision.
//!
//! The session starts from an empty visible state: the operator knows no refs,
//! dimensions, or cursor yet and must open a bounded wake window
//! (`budget.max_entries`) and then widen coverage with `kernel_near` /
//! `kernel_forward` / `kernel_rewind` itself. The period, the bounded-window
//! size, and the "widen until covered" instruction are carried in the `goal`,
//! not the envelope, so the expansion schedule stays the policy's job. The
//! visible budget snapshot is derived from the same session budget so the first
//! subject the policy sees reports the real remaining calls.

use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::error::runtime_domain_result::RuntimeDomainResult;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;

#[derive(Debug)]
pub struct WindowExpansionEpisodeCompiler;

impl WindowExpansionEpisodeCompiler {
    /// Build the runtime request for a window-expansion session. `token_budget`
    /// caps total tokens across the session's calls; the call budget is derived
    /// from `spec.required_calls()`.
    pub fn compile(
        session_id: OperatorSessionId,
        about: AboutId,
        goal: TrajectoryGoal,
        spec: &WindowExpansionSpec,
        token_budget: u32,
    ) -> RuntimeDomainResult<OperatorRequest> {
        let budget = SessionBudget::new(spec.required_calls(), token_budget);
        let initial_state = VisibleState::assemble([], [], None, budget_snapshot(budget));
        OperatorRequest::new(
            session_id,
            goal,
            initial_state,
            OperatorMode::Read,
            AllowedTools::for_mode(OperatorMode::Read),
            budget,
            about,
            None,
        )
    }
}

fn budget_snapshot(budget: SessionBudget) -> BudgetSnapshot {
    BudgetSnapshot::bounded(
        budget.calls_remaining() as usize,
        budget.tokens_remaining() as usize,
    )
}
