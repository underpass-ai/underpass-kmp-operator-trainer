//! End-to-end test for the cross-about count generator: drive the multi-step
//! loop with a scripted teacher across several abouts and assert the generated
//! SFT trajectories are attributed to the about each step ran in, and that an
//! episode leaving an about short of its gold is dropped.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_application::use_cases::generate_cross_about_expansions_use_case::GenerateCrossAboutExpansionsUseCase;
use operator_runtime_application::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::navigation_signals::NavigationSignals;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;
use operator_synthetic_domain::episode::cross_about_target::CrossAboutTarget;
use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;

#[derive(Debug)]
struct ScriptedPolicy {
    actions: Mutex<VecDeque<OperatorAction>>,
    fallback: OperatorAction,
}

impl OperatorPolicy for ScriptedPolicy {
    fn predict(&self, _s: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError> {
        Ok(self
            .actions
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct SeqExecutor {
    observations: Mutex<VecDeque<Observation>>,
    fallback: Observation,
}

impl McpExecutor for SeqExecutor {
    fn execute(
        &self,
        _action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        Ok(self
            .observations
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct NoopSink;

impl SessionEventSink for NoopSink {
    fn on_request_received(&self, _request: &OperatorRequest) {}
    fn on_action_predicted(&self, _action: &OperatorAction) {}
    fn on_observation(&self, _observation: &Observation) {}
    fn on_session_complete(&self, _outcome: &SessionOutcome) {}
}

fn text(value: &str) -> NonEmptyString {
    NonEmptyString::parse(value, "xab_test").unwrap()
}

fn memory_ref(value: &str) -> MemoryRef {
    MemoryRef::parse(value).unwrap()
}

fn wake(about: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
        WakeArguments::new(AboutId::parse(about).unwrap()),
    )))
}

fn stop() -> OperatorAction {
    OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
}

fn response(target: &str) -> Observation {
    Observation::ToolResponse {
        outcome: ToolOutcome::Inspect(InspectOutcome::new(
            text("inspect"),
            memory_ref(target),
            text("node"),
            vec![],
            vec![],
        )),
        observed_refs: vec![memory_ref(target)],
        signals: NavigationSignals::new(4, 0, false, 1, 1, 0, 0, false, false, 0, 0),
    }
}

fn episode(targets: &[(&str, &str)]) -> CrossAboutEpisode {
    CrossAboutEpisode::new(
        targets
            .iter()
            .map(|(about, gold)| {
                CrossAboutTarget::new(AboutId::parse(*about).unwrap(), vec![memory_ref(gold)])
            })
            .collect(),
        TrajectoryGoal::parse("Count workshops across EU and US.").unwrap(),
        WindowExpansionSpec::new(
            PositiveCount::parse(4, "window").unwrap(),
            PositiveCount::parse(3, "iterations").unwrap(),
        ),
        4096,
    )
}

fn generator(
    actions: Vec<OperatorAction>,
    observations: Vec<Observation>,
) -> GenerateCrossAboutExpansionsUseCase {
    let session = RunOperatorSessionMultiStepUseCase::new(
        Arc::new(ScriptedPolicy {
            actions: Mutex::new(actions.into()),
            fallback: stop(),
        }),
        Arc::new(SeqExecutor {
            observations: Mutex::new(observations.into()),
            fallback: response("node:z"),
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSink),
    );
    GenerateCrossAboutExpansionsUseCase::new(
        session,
        TaskFamily::parse("runtime.cross_about_count").unwrap(),
    )
}

#[test]
fn generates_cross_about_trajectories_attributed_to_each_about() {
    let report = generator(
        vec![wake("about:eu"), wake("about:us"), stop()],
        vec![response("node:eu1"), response("node:us1")],
    )
    .execute(&[episode(&[
        ("about:eu", "node:eu1"),
        ("about:us", "node:us1"),
    ])])
    .expect("generation runs");

    assert_eq!(report.accepted_episodes(), 1);
    assert_eq!(report.dropped_episodes(), 0);
    // wake(eu) is chosen in the entry about; wake(us) is chosen while still in
    // eu (the switch lands afterwards); the terminal stop runs in us.
    let abouts: Vec<&str> = report
        .trajectories()
        .iter()
        .map(|t| t.about().as_str())
        .collect();
    assert_eq!(abouts, ["about:eu", "about:eu", "about:us"]);
}

#[test]
fn drops_an_episode_that_left_an_about_short() {
    // The teacher never wakes US, so node:us1 is never retrieved: gold coverage
    // fails for about:us and the episode is dropped rather than taught.
    let report = generator(vec![wake("about:eu"), stop()], vec![response("node:eu1")])
        .execute(&[episode(&[
            ("about:eu", "node:eu1"),
            ("about:us", "node:us1"),
        ])])
        .expect("generation runs");

    assert_eq!(report.accepted_episodes(), 0);
    assert_eq!(report.dropped_episodes(), 1);
    assert_eq!(report.trajectories().len(), 0);
    assert_eq!(report.drops()[0].reason(), "uncovered_abouts:about:us");
}
