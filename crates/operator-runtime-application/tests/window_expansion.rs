//! Tests for the window-expansion generator machinery: the transcript->
//! trajectory expander, the coverage oracle, and the episode compiler. The
//! expander tests drive the real multi-step loop so they exercise the exact
//! `perceived_state` the loop records (the property the expander relies on); the
//! oracle's edge cases use hand-built transcripts for precise control over the
//! final signals.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_application::services::window_expansion_episode_compiler::WindowExpansionEpisodeCompiler;
use operator_runtime_application::services::window_expansion_oracle::WindowExpansionOracle;
use operator_runtime_application::use_cases::expand_session_transcript_use_case::ExpandSessionTranscriptUseCase;
use operator_runtime_application::use_cases::generate_window_expansions_use_case::GenerateWindowExpansionsUseCase;
use operator_runtime_application::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;
use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::execution_step::ExecutionStep;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::observation_error_code::ObservationErrorCode;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_domain::session::session_transcript::SessionTranscript;
use operator_runtime_domain::session::window_coverage_outcome::WindowCoverageOutcome;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::coverage_deviation_snapshot::CoverageDeviationSnapshot;
use operator_shared_domain::visible_state::navigation_signals::NavigationSignals;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::episode::window_expansion_episode::WindowExpansionEpisode;
use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;

// ----- shared fixtures -------------------------------------------------------

#[derive(Debug)]
struct ScriptedOperatorPolicy {
    actions: Mutex<VecDeque<OperatorAction>>,
    fallback: OperatorAction,
}

impl OperatorPolicy for ScriptedOperatorPolicy {
    fn predict(
        &self,
        _subject: &CalibrationSubject,
    ) -> Result<OperatorAction, OperatorPolicyError> {
        let mut queue = self.actions.lock().unwrap();
        Ok(queue.pop_front().unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct SeqMcpExecutor {
    observations: Mutex<VecDeque<Observation>>,
    fallback: Observation,
    calls: Arc<AtomicUsize>,
}

impl McpExecutor for SeqMcpExecutor {
    fn execute(
        &self,
        _action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut queue = self.observations.lock().unwrap();
        Ok(queue.pop_front().unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct NoopSessionEventSink;

impl SessionEventSink for NoopSessionEventSink {
    fn on_request_received(&self, _request: &OperatorRequest) {}
    fn on_action_predicted(&self, _action: &OperatorAction) {}
    fn on_observation(&self, _observation: &Observation) {}
    fn on_session_complete(
        &self,
        _outcome: &operator_runtime_domain::session::session_outcome::SessionOutcome,
    ) {
    }
}

fn text(value: &str) -> NonEmptyString {
    NonEmptyString::parse(value, "window_expansion_test").unwrap()
}

fn memory_ref(value: &str) -> MemoryRef {
    MemoryRef::parse(value).unwrap()
}

fn inspect_action(target: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(memory_ref(target)),
    )))
}

fn stop_action() -> OperatorAction {
    OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
}

/// A successful inspect observation surfacing `target`, with explicit coverage
/// signals so the oracle and the deviation accumulator have something to read.
fn response(target: &str, signals: NavigationSignals) -> Observation {
    Observation::ToolResponse {
        outcome: ToolOutcome::Inspect(InspectOutcome::new(
            text("inspect"),
            memory_ref(target),
            text("node"),
            vec![],
            vec![],
        )),
        observed_refs: vec![memory_ref(target)],
        signals,
    }
}

fn error_observation() -> Observation {
    Observation::ToolError {
        code: ObservationErrorCode::parse("kernel_rejected").unwrap(),
        message: "kernel rejected request".to_string(),
    }
}

/// Builds navigation signals from the three fields the oracle reads
/// (`coverage_missing`, `page_has_more`, `frontier_size`); the rest are neutral.
fn signals(coverage_missing: u32, page_has_more: bool, frontier_size: u32) -> NavigationSignals {
    NavigationSignals::new(
        4,
        coverage_missing,
        page_has_more,
        1,
        1,
        frontier_size,
        0,
        false,
        false,
        0,
        0,
    )
}

fn covered_signals() -> NavigationSignals {
    signals(0, false, 0)
}

fn task_family() -> TaskFamily {
    TaskFamily::parse("runtime.window_expansion").unwrap()
}

fn request(calls_remaining: u32) -> OperatorRequest {
    OperatorRequest::new(
        OperatorSessionId::parse("session:we").unwrap(),
        TrajectoryGoal::parse("Count workshops across the period by widening the window.").unwrap(),
        VisibleState::assemble(
            [memory_ref("node:1")],
            [],
            None,
            BudgetSnapshot::bounded(1, 4096),
        ),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(calls_remaining, 4096),
        AboutId::parse("about:test").unwrap(),
        None,
    )
    .unwrap()
}

fn run(
    actions: Vec<OperatorAction>,
    observations: Vec<Observation>,
    calls_remaining: u32,
) -> (OperatorRequest, SessionTranscript) {
    let request = request(calls_remaining);
    let use_case = RunOperatorSessionMultiStepUseCase::new(
        Arc::new(ScriptedOperatorPolicy {
            actions: Mutex::new(actions.into()),
            fallback: stop_action(),
        }),
        Arc::new(SeqMcpExecutor {
            observations: Mutex::new(observations.into()),
            fallback: response("node:9", covered_signals()),
            calls: Arc::new(AtomicUsize::new(0)),
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    );
    let transcript = use_case.execute(&request).expect("session runs");
    (request, transcript)
}

// ----- expander --------------------------------------------------------------

#[test]
fn expands_each_tool_step_and_the_terminal_stop() {
    let (request, transcript) = run(
        vec![
            inspect_action("node:1"),
            inspect_action("node:2"),
            stop_action(),
        ],
        vec![
            response("node:2", signals(2, true, 1)),
            response("node:3", covered_signals()),
        ],
        5,
    );

    let trajectories =
        ExpandSessionTranscriptUseCase::expand(&request, &transcript, &task_family())
            .expect("expansion succeeds");

    // Two tool steps plus the terminal stop.
    assert_eq!(trajectories.len(), 3);
    assert!(matches!(
        trajectories[2].target_action(),
        OperatorAction::Stop(_)
    ));
    // Each trajectory carries the exact state the policy perceived for it: the
    // step trajectories mirror the recorded steps, the terminal mirrors the
    // final state. This is the property that makes expansion replay-free.
    assert_eq!(
        trajectories[0].visible_state(),
        transcript.steps()[0].perceived_state()
    );
    assert_eq!(
        trajectories[1].visible_state(),
        transcript.steps()[1].perceived_state()
    );
    assert_eq!(
        trajectories[2].visible_state(),
        transcript.final_visible_state()
    );
    // Ids are session-prefixed and unique.
    let ids: Vec<&str> = trajectories.iter().map(|t| t.id().as_str()).collect();
    assert_eq!(
        ids,
        ["session:we:0000", "session:we:0001", "session:we:0002"]
    );
    assert_eq!(trajectories[0].step_id().as_str(), "step:session:we:0000");
    assert_eq!(trajectories[0].task_family(), &task_family());
}

#[test]
fn skips_tool_error_steps() {
    // Step 0 errors (no refs), step 1 succeeds, then stop.
    let (request, transcript) = run(
        vec![
            inspect_action("node:1"),
            inspect_action("node:1"),
            stop_action(),
        ],
        vec![error_observation(), response("node:2", covered_signals())],
        5,
    );
    assert_eq!(transcript.step_count(), 2);

    let trajectories =
        ExpandSessionTranscriptUseCase::expand(&request, &transcript, &task_family())
            .expect("expansion succeeds");

    // The error step is dropped: only the successful step plus the terminal.
    assert_eq!(trajectories.len(), 2);
    assert!(matches!(
        trajectories[1].target_action(),
        OperatorAction::Stop(_)
    ));
}

#[test]
fn does_not_emit_a_terminal_for_a_budget_exhausted_session() {
    // Budget of 2 exhausts before the third inspect, so the terminal action
    // never ran and must not become a target.
    let (request, transcript) = run(
        vec![
            inspect_action("node:1"),
            inspect_action("node:2"),
            inspect_action("node:3"),
        ],
        vec![
            response("node:2", signals(2, true, 1)),
            response("node:3", signals(1, true, 1)),
        ],
        2,
    );

    let trajectories =
        ExpandSessionTranscriptUseCase::expand(&request, &transcript, &task_family())
            .expect("expansion succeeds");

    // Two tool steps, no terminal trajectory.
    assert_eq!(trajectories.len(), 2);
    assert!(
        trajectories
            .iter()
            .all(|t| matches!(t.target_action(), OperatorAction::ToolCall(_)))
    );
}

#[test]
fn emits_the_terminal_escalate_for_an_escalated_session() {
    let escalate = OperatorAction::Escalate(EscalateAction::new(
        EscalateReason::BeyondCapability,
        ModelId::parse("claude-opus-4-7").unwrap(),
    ));
    let (request, transcript) = run(
        vec![inspect_action("node:1"), escalate.clone()],
        vec![response("node:2", signals(2, true, 1))],
        5,
    );

    let trajectories =
        ExpandSessionTranscriptUseCase::expand(&request, &transcript, &task_family())
            .expect("expansion succeeds");

    // One tool step plus the terminal escalate.
    assert_eq!(trajectories.len(), 2);
    assert!(matches!(
        trajectories[1].target_action(),
        OperatorAction::Escalate(_)
    ));
}

#[test]
fn surfaces_a_trajectory_error_for_a_terminal_action_outside_the_mode() {
    // A hand-built (loop-impossible) transcript whose terminal action uses a
    // write tool in a read session exercises the defensive trajectory-build
    // error path the loop would never produce.
    let write_terminal = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
        WriteMemoryArguments::new("summary", "body", vec![]).unwrap(),
    )));
    let transcript = SessionTranscript::completed(
        OperatorSessionId::parse("session:bad").unwrap(),
        Vec::new(),
        write_terminal,
        VisibleState::assemble([], [], None, BudgetSnapshot::bounded(1, 4096)),
        SessionBudget::new(1, 4096),
        Duration::from_millis(1),
        AboutId::parse("about:test").unwrap(),
    );

    let error = ExpandSessionTranscriptUseCase::expand(&request(5), &transcript, &task_family())
        .expect_err("a write terminal in a read session is rejected");

    assert!(matches!(
        error,
        operator_runtime_application::errors::expand_session_transcript_error::ExpandSessionTranscriptError::Trajectory { .. }
    ));
}

// ----- oracle ----------------------------------------------------------------

fn transcript_with(steps: Vec<ExecutionStep>, final_state: VisibleState) -> SessionTranscript {
    SessionTranscript::completed(
        OperatorSessionId::parse("session:oracle").unwrap(),
        steps,
        stop_action(),
        final_state,
        SessionBudget::new(1, 4096),
        Duration::from_millis(5),
        AboutId::parse("about:test").unwrap(),
    )
}

fn step_with(observation: Observation) -> ExecutionStep {
    ExecutionStep::new(
        inspect_action("node:1"),
        observation,
        VisibleState::assemble(
            [memory_ref("node:1")],
            [],
            None,
            BudgetSnapshot::bounded(1, 4096),
        ),
        AboutId::parse("about:test").unwrap(),
    )
}

fn final_state(deviation: CoverageDeviationSnapshot) -> VisibleState {
    VisibleState::assemble(
        [memory_ref("node:1")],
        [],
        None,
        BudgetSnapshot::bounded(1, 4096),
    )
    .with_coverage_deviation(deviation)
}

#[test]
fn oracle_reports_complete_when_last_move_is_fully_covered() {
    let transcript = transcript_with(
        vec![step_with(response("node:2", covered_signals()))],
        final_state(CoverageDeviationSnapshot::new(50, false, false)),
    );
    assert_eq!(
        WindowExpansionOracle::verify(&transcript),
        WindowCoverageOutcome::Complete
    );
}

#[test]
fn oracle_reports_incomplete_when_shortfall_remains_without_saturation() {
    let transcript = transcript_with(
        vec![step_with(response("node:2", signals(2, true, 3)))],
        final_state(CoverageDeviationSnapshot::new(700, false, false)),
    );
    assert_eq!(
        WindowExpansionOracle::verify(&transcript),
        WindowCoverageOutcome::Incomplete { remaining: 5 }
    );
}

#[test]
fn oracle_reports_saturated_when_stalled_with_shortfall() {
    let transcript = transcript_with(
        vec![step_with(response("node:2", signals(1, true, 1)))],
        final_state(CoverageDeviationSnapshot::new(600, true, false)),
    );
    assert_eq!(
        WindowExpansionOracle::verify(&transcript),
        WindowCoverageOutcome::Saturated
    );
}

#[test]
fn oracle_reports_incomplete_when_no_temporal_move_happened() {
    let transcript = transcript_with(
        vec![step_with(error_observation())],
        final_state(CoverageDeviationSnapshot::unknown()),
    );
    assert_eq!(
        WindowExpansionOracle::verify(&transcript),
        WindowCoverageOutcome::Incomplete { remaining: 0 }
    );
}

// ----- compiler --------------------------------------------------------------

#[test]
fn compiler_sizes_a_read_request_to_the_spec() {
    let spec = WindowExpansionSpec::new(
        PositiveCount::parse(8, "window").unwrap(),
        PositiveCount::parse(5, "iterations").unwrap(),
    );
    let compiled = WindowExpansionEpisodeCompiler::compile(
        OperatorSessionId::parse("session:compiled").unwrap(),
        AboutId::parse("about:period").unwrap(),
        TrajectoryGoal::parse("Widen the window until the period is covered.").unwrap(),
        &spec,
        4096,
    )
    .expect("compile succeeds");

    assert_eq!(compiled.mode(), OperatorMode::Read);
    assert_eq!(
        compiled.allowed_tools(),
        &AllowedTools::for_mode(OperatorMode::Read)
    );
    // required_calls = max_iterations + 1 (a terminal decision).
    assert_eq!(compiled.initial_budget().calls_remaining(), 6);
    // The session starts from an empty visible state the policy must fill.
    assert!(compiled.initial_visible_state().known_refs().is_empty());
    assert!(compiled.initial_visible_state().active_cursor().is_none());
}

// ----- generator orchestrator ------------------------------------------------

fn session_use_case(
    actions: Vec<OperatorAction>,
    observations: Vec<Observation>,
) -> RunOperatorSessionMultiStepUseCase {
    RunOperatorSessionMultiStepUseCase::new(
        Arc::new(ScriptedOperatorPolicy {
            actions: Mutex::new(actions.into()),
            fallback: stop_action(),
        }),
        Arc::new(SeqMcpExecutor {
            observations: Mutex::new(observations.into()),
            fallback: response("node:9", covered_signals()),
            calls: Arc::new(AtomicUsize::new(0)),
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    )
}

// `Ask` is exempt from the known-entities contract, so it is a valid first move
// from the empty state the compiler builds (just like Rewind/Forward).
fn ask_action() -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
        AskArguments::new("Count workshops over the period.").unwrap(),
    )))
}

fn episode(about: &str, max_iterations: usize) -> WindowExpansionEpisode {
    WindowExpansionEpisode::new(
        AboutId::parse(about).unwrap(),
        TrajectoryGoal::parse("Count workshops across the period by widening the window.").unwrap(),
        WindowExpansionSpec::new(
            PositiveCount::parse(4, "window").unwrap(),
            PositiveCount::parse(max_iterations, "iterations").unwrap(),
        ),
        4096,
    )
}

#[test]
fn generator_accepts_a_covered_episode_into_trajectories() {
    let session = session_use_case(
        vec![ask_action(), stop_action()],
        vec![response("node:1", covered_signals())],
    );
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    let report = generator
        .execute(&[episode("about:p", 5)])
        .expect("generation succeeds");

    assert_eq!(report.accepted_episodes(), 1);
    assert_eq!(report.dropped_episodes(), 0);
    // One ask step plus the terminal stop.
    assert_eq!(report.trajectories().len(), 2);
    assert!(report.drop_rate().abs() < f64::EPSILON);
}

#[test]
fn generator_drops_episodes_that_never_reach_coverage() {
    // Empty action queue + stop fallback: every episode stops immediately with
    // no temporal move, so the oracle reports it uncovered and it is dropped.
    let session = session_use_case(vec![], vec![]);
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    let report = generator
        .execute(&[episode("about:a", 3), episode("about:b", 3)])
        .expect("generation succeeds");

    assert_eq!(report.accepted_episodes(), 0);
    assert_eq!(report.dropped_episodes(), 2);
    assert!(report.trajectories().is_empty());
    assert!((report.drop_rate() - 1.0).abs() < f64::EPSILON);
    assert_eq!(report.drops()[0].reason(), "incomplete");
}

// Fully-covered shape but with a surfaced conflict (conflicts = 1).
fn covered_but_conflicted_signals() -> NavigationSignals {
    NavigationSignals::new(4, 0, false, 1, 1, 0, 1, false, false, 0, 0)
}

#[test]
fn generator_drops_an_episode_that_surfaces_a_conflict() {
    let session = session_use_case(
        vec![ask_action(), stop_action()],
        vec![response("node:1", covered_but_conflicted_signals())],
    );
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    let report = generator
        .execute(&[episode("about:c", 5)])
        .expect("generation succeeds");

    // Coverage is complete, but the conflict blocks acceptance.
    assert_eq!(report.accepted_episodes(), 0);
    assert_eq!(report.dropped_episodes(), 1);
    assert_eq!(report.drops()[0].reason(), "conflict_blocking");
}

#[derive(Debug)]
struct FailingMcpExecutor;

impl McpExecutor for FailingMcpExecutor {
    fn execute(
        &self,
        _action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        Err(McpExecutorError::Transport {
            message: "kernel unreachable".to_string(),
        })
    }
}

#[test]
fn generator_propagates_an_executor_failure() {
    let session = RunOperatorSessionMultiStepUseCase::new(
        Arc::new(ScriptedOperatorPolicy {
            actions: Mutex::new(vec![ask_action()].into()),
            fallback: stop_action(),
        }),
        Arc::new(FailingMcpExecutor),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    );
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    let error = generator
        .execute(&[episode("about:e", 3)])
        .expect_err("executor failure propagates");

    assert!(matches!(
        error,
        operator_runtime_application::errors::generate_window_expansions_error::GenerateWindowExpansionsError::Session { index: 0, .. }
    ));
}

fn gold_episode(about: &str, expected: &[&str]) -> WindowExpansionEpisode {
    episode(about, 5).with_expected_refs(expected.iter().map(|r| memory_ref(r)).collect())
}

#[test]
fn generator_accepts_when_the_gold_period_set_is_retrieved() {
    let session = session_use_case(
        vec![ask_action(), stop_action()],
        vec![response("node:1", covered_signals())],
    );
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    let report = generator
        .execute(&[gold_episode("about:g", &["node:1"])])
        .expect("generation succeeds");

    assert_eq!(report.accepted_episodes(), 1);
    assert_eq!(report.dropped_episodes(), 0);
}

#[test]
fn generator_drops_when_a_gold_ref_is_not_retrieved() {
    let session = session_use_case(
        vec![ask_action(), stop_action()],
        vec![response("node:1", covered_signals())],
    );
    let generator = GenerateWindowExpansionsUseCase::new(session, task_family());

    // node:2 is in the gold set but the session only ever retrieves node:1, so
    // the gold gate rejects it even though the signals read as covered.
    let report = generator
        .execute(&[gold_episode("about:g", &["node:1", "node:2"])])
        .expect("generation succeeds");

    assert_eq!(report.accepted_episodes(), 0);
    assert_eq!(report.dropped_episodes(), 1);
    assert_eq!(report.drops()[0].reason(), "missing_gold_refs");
}
