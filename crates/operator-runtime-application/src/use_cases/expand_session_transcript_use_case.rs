//! Expand a finished multi-step [`SessionTranscript`] into the SFT
//! [`TrainingTrajectory`] records it demonstrates.
//!
//! This is the bridge from *execution* (a runtime session the operator ran)
//! to *training data* (one `(prompt, target)` example per decision). It is the
//! piece that makes a window-expansion session usable for SFT: each step the
//! policy took becomes a trajectory whose visible state — including the
//! online `coverage_deviation` the policy perceived at that point — is the
//! prompt, and whose action is the target the student should learn to predict.
//!
//! Correctness rests on [`crate::session`-recorded][`ExecutionStep`]
//! `perceived_state`: the loop already captured the exact state the policy saw
//! for each step, so expansion never replays the loop's state threading and
//! cannot drift an off-by-one between the state before vs. after a step.
//!
//! Two rules keep the corpus clean:
//! - tool-error steps are skipped (their action led to an executor failure and
//!   must not be taught as a target), and
//! - the terminal action is emitted only when the session ended cleanly
//!   (`Completed`/`Escalated`); budget-exhausted and contract-violation
//!   terminals are dropped because that action never ran or was rejected.
//!
//! The use case is policy-agnostic and label-free: it never consults an oracle
//! or injects an "expected" action, so whatever the teacher demonstrated is
//! exactly what the student learns. Accepting or dropping a whole transcript is
//! the caller's job (see the window-expansion oracle).

use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::outcome_class::OutcomeClass;
use operator_runtime_domain::session::session_transcript::SessionTranscript;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::errors::expand_session_transcript_error::ExpandSessionTranscriptError;

#[derive(Debug)]
pub struct ExpandSessionTranscriptUseCase;

impl ExpandSessionTranscriptUseCase {
    /// Build one [`TrainingTrajectory`] per non-error step plus, when the
    /// session ended cleanly, the terminal decision. `task_family` labels the
    /// emitted trajectories (the runtime loop itself is family-agnostic, so the
    /// caller names the family, e.g. `runtime.window_expansion`). Session-id
    /// prefixed identifiers keep step ids globally unique across sessions.
    pub fn expand(
        request: &OperatorRequest,
        transcript: &SessionTranscript,
        task_family: &TaskFamily,
    ) -> Result<Vec<TrainingTrajectory>, ExpandSessionTranscriptError> {
        let session = transcript.session_id().as_str();
        let mut trajectories = Vec::new();
        let mut index = 0usize;

        for step in transcript.steps() {
            if !matches!(step.observation(), Observation::ToolResponse { .. }) {
                continue;
            }
            trajectories.push(build_trajectory(
                session,
                index,
                request,
                task_family,
                step.perceived_state(),
                step.action(),
                step.about(),
            )?);
            index += 1;
        }

        if matches!(
            transcript.outcome_class(),
            OutcomeClass::Completed | OutcomeClass::Escalated
        ) {
            trajectories.push(build_trajectory(
                session,
                index,
                request,
                task_family,
                transcript.final_visible_state(),
                transcript.terminal_action(),
                transcript.terminal_about(),
            )?);
        }

        Ok(trajectories)
    }
}

#[allow(clippy::too_many_arguments)]
fn build_trajectory(
    session: &str,
    index: usize,
    request: &OperatorRequest,
    task_family: &TaskFamily,
    visible_state: &VisibleState,
    action: &OperatorAction,
    about: &AboutId,
) -> Result<TrainingTrajectory, ExpandSessionTranscriptError> {
    let trajectory_id =
        TrainingTrajectoryId::parse(format!("{session}:{index:04}")).map_err(|err| {
            ExpandSessionTranscriptError::Id {
                message: err.to_string(),
            }
        })?;
    let step_id = StepId::parse(format!("step:{session}:{index:04}")).map_err(|err| {
        ExpandSessionTranscriptError::Id {
            message: err.to_string(),
        }
    })?;
    TrainingTrajectory::new(
        trajectory_id,
        step_id,
        about.clone(),
        request.mode(),
        task_family.clone(),
        request.goal().clone(),
        request.allowed_tools().clone(),
        visible_state.clone(),
        action.clone(),
    )
    .map_err(|err| ExpandSessionTranscriptError::Trajectory {
        message: err.to_string(),
    })
}
