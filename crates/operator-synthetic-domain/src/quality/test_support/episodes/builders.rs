//! Typed builders shared by the handcrafted v7.2 episodes.

use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::ingest_confidence::IngestConfidence;
use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
use operator_shared_domain::tool_arguments::ingest_evidence::IngestEvidence;
use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
use operator_shared_domain::tool_arguments::ingest_relation::IngestRelation;
use operator_shared_domain::tool_arguments::ingest_relation_class::IngestRelationClass;
use operator_shared_domain::tool_arguments::ingest_relation_type::IngestRelationType;
use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::string_map::StringMap;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::capability_target::CapabilityTarget;
use crate::episode::episode_id::EpisodeId;
use crate::episode::episode_objective::EpisodeObjective;
use crate::episode::episode_step_plan::EpisodeStepPlan;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

pub(super) fn episode_spec(
    id: &str,
    theme: EpisodeTheme,
    objective: &str,
    capabilities: Vec<KmpMcpCapability>,
) -> SyntheticEpisodeSpec {
    SyntheticEpisodeSpec::new(
        EpisodeId::parse(id).unwrap(),
        theme,
        EpisodeObjective::parse(objective).unwrap(),
        capabilities
            .into_iter()
            .map(|capability| step(capability, "Exercise the KMP/MCP capability in context."))
            .collect(),
    )
    .unwrap()
}

pub(super) fn refs(about: &str, labels: &[&str]) -> Vec<MemoryRef> {
    labels
        .iter()
        .map(|label| memory_ref(about, label))
        .collect()
}

pub(super) fn dims(labels: &[&str]) -> Vec<DimensionRef> {
    labels.iter().map(|label| dimension_ref(label)).collect()
}

pub(super) fn wake(about: &str) -> ToolArguments {
    ToolArguments::Wake(WakeArguments::new(about_id(about)))
}

pub(super) fn ask(question: &str) -> ToolArguments {
    ToolArguments::Ask(AskArguments::new(question).unwrap())
}

pub(super) fn near(anchor: MemoryRef, dimension: DimensionRef) -> ToolArguments {
    ToolArguments::Near(
        NearArguments::new(
            anchor,
            vec![dimension],
            Some(PositiveCount::parse(3, "limit").unwrap()),
        )
        .unwrap(),
    )
}

pub(super) fn goto(target: MemoryRef) -> ToolArguments {
    ToolArguments::Goto(GotoArguments::new(Cursor::Ref(RefCursor::new(target))))
}

pub(super) fn rewind(index: usize) -> ToolArguments {
    ToolArguments::Rewind(RewindArguments::new(
        temporal_cursor(index),
        PositiveCount::parse(2, "window").unwrap(),
    ))
}

pub(super) fn forward(index: usize) -> ToolArguments {
    ToolArguments::Forward(ForwardArguments::new(
        temporal_cursor(index),
        PositiveCount::parse(2, "window").unwrap(),
    ))
}

pub(super) fn trace(from: MemoryRef, to: MemoryRef) -> ToolArguments {
    ToolArguments::Trace(TraceArguments::new(
        from,
        Some(to),
        PositiveCount::parse(8, "page").unwrap(),
    ))
}

pub(super) fn inspect(target: MemoryRef) -> ToolArguments {
    ToolArguments::Inspect(InspectArguments::new(target))
}

pub(super) fn active_temporal_cursor(index: usize) -> Cursor {
    Cursor::Temporal(temporal_cursor(index))
}

pub(super) fn write_memory(
    summary: &str,
    rationale: &str,
    evidence: Vec<MemoryRef>,
) -> ToolArguments {
    ToolArguments::WriteMemory(WriteMemoryArguments::new(summary, rationale, evidence).unwrap())
}

pub(super) fn rich_ingest(
    about: &str,
    target: MemoryRef,
    dimension: &DimensionRef,
    index: usize,
) -> ToolArguments {
    ingest_with_relation(
        about,
        target,
        dimension,
        index,
        "chosen_because",
        IngestRelationClass::Causal,
        IngestConfidence::High,
        "The inspected evidence explains why the new decision was chosen.",
        "The candidate write is supported by the inspected evidence node.",
    )
}

pub(super) fn anemic_ingest(
    about: &str,
    target: MemoryRef,
    dimension: &DimensionRef,
    index: usize,
) -> ToolArguments {
    ingest_with_relation(
        about,
        target,
        dimension,
        index,
        "follows",
        IngestRelationClass::Procedural,
        IngestConfidence::Medium,
        "No stronger causal or evidential relation is visible.",
        "Only process order is available, so the fallback relation is explicit.",
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn read_tool(
    about: &str,
    index: usize,
    family: &str,
    goal: &str,
    refs: Vec<MemoryRef>,
    dims: Vec<DimensionRef>,
    active_cursor: Option<Cursor>,
    calls_remaining: usize,
    arguments: ToolArguments,
) -> TrainingTrajectory {
    trajectory(
        about,
        index,
        OperatorMode::Read,
        family,
        goal,
        refs,
        dims,
        active_cursor,
        calls_remaining,
        OperatorAction::ToolCall(ToolCallAction::new(arguments)),
    )
}

pub(super) fn write_tool(
    about: &str,
    index: usize,
    family: &str,
    goal: &str,
    refs: Vec<MemoryRef>,
    dims: Vec<DimensionRef>,
    arguments: ToolArguments,
) -> TrainingTrajectory {
    trajectory(
        about,
        index,
        OperatorMode::Write,
        family,
        goal,
        refs,
        dims,
        None,
        1,
        OperatorAction::ToolCall(ToolCallAction::new(arguments)),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stop_row(
    about: &str,
    index: usize,
    mode: OperatorMode,
    family: &str,
    goal: &str,
    refs: Vec<MemoryRef>,
    dims: Vec<DimensionRef>,
    reason: StopReason,
    answer: Option<&str>,
    evidence: Vec<MemoryRef>,
    calls_remaining: usize,
) -> TrainingTrajectory {
    trajectory(
        about,
        index,
        mode,
        family,
        goal,
        refs,
        dims,
        None,
        calls_remaining,
        OperatorAction::Stop(
            StopAction::new(reason, answer.map(str::to_string), evidence).unwrap(),
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn escalate_row(
    about: &str,
    index: usize,
    family: &str,
    goal: &str,
    refs: Vec<MemoryRef>,
    dims: Vec<DimensionRef>,
    reason: EscalateReason,
    calls_remaining: usize,
) -> TrainingTrajectory {
    trajectory(
        about,
        index,
        OperatorMode::Read,
        family,
        goal,
        refs,
        dims,
        None,
        calls_remaining,
        OperatorAction::Escalate(EscalateAction::new(
            reason,
            ModelId::parse("frontier-reasoner").unwrap(),
        )),
    )
}

pub(in crate::quality::test_support) fn inspect_only_trajectory(
    about: &str,
    index: usize,
    step_id: &str,
) -> TrainingTrajectory {
    let target = memory_ref(about, "only");
    let dim = dimension_ref("agent:operator");
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse(format!("trajectory:{about}:{index}")).unwrap(),
        StepId::parse(step_id).unwrap(),
        about_id(about),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        TrajectoryGoal::parse("Inspect a single visible node.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        VisibleState::assemble(
            [target.clone()],
            [dim],
            None,
            BudgetSnapshot::bounded(1, 1024),
        ),
        OperatorAction::ToolCall(ToolCallAction::new(inspect(target))),
    )
    .unwrap()
}

fn step(capability: KmpMcpCapability, objective: &str) -> EpisodeStepPlan {
    EpisodeStepPlan::new(
        CapabilityTarget::new(capability, PositiveCount::parse(1, "minimum").unwrap()),
        EpisodeObjective::parse(objective).unwrap(),
    )
}

#[allow(clippy::too_many_arguments)]
fn trajectory(
    about: &str,
    index: usize,
    mode: OperatorMode,
    family: &str,
    goal: &str,
    refs: Vec<MemoryRef>,
    dims: Vec<DimensionRef>,
    active_cursor: Option<Cursor>,
    calls_remaining: usize,
    action: OperatorAction,
) -> TrainingTrajectory {
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse(format!("trajectory:{about}:{index:02}")).unwrap(),
        StepId::parse(format!("step:{about}:{index:02}")).unwrap(),
        about_id(about),
        mode,
        TaskFamily::parse(format!("{}.{}", mode.as_str(), family)).unwrap(),
        TrajectoryGoal::parse(goal).unwrap(),
        AllowedTools::for_mode(mode),
        VisibleState::assemble(
            refs,
            dims,
            active_cursor,
            BudgetSnapshot::bounded(calls_remaining, 4096),
        ),
        action,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn ingest_with_relation(
    about: &str,
    target: MemoryRef,
    dimension: &DimensionRef,
    index: usize,
    relation: &str,
    relation_class: IngestRelationClass,
    confidence: IngestConfidence,
    why: &str,
    evidence_text: &str,
) -> ToolArguments {
    let entry = memory_ref(about, &format!("new-{index}"));
    let coordinate = IngestTemporalCoordinate::new(
        dimension.clone(),
        NonEmptyString::parse("scope:writer", "scope").unwrap(),
        None,
        None,
        None,
        None,
        None,
        Some(PositiveCount::parse(index, "sequence").unwrap()),
        None,
        StringMap::empty(),
    )
    .unwrap();
    let relation = IngestRelation::new(
        entry.clone(),
        target.clone(),
        IngestRelationType::parse(relation).unwrap(),
        relation_class,
        Some(NonEmptyString::parse(why, "why").unwrap()),
        Some(NonEmptyString::parse(evidence_text, "evidence").unwrap()),
        Some(confidence),
        Some(PositiveCount::parse(index, "sequence").unwrap()),
    )
    .unwrap();
    let evidence = IngestEvidence::new(
        NonEmptyString::parse(format!("evidence:{about}:{index}"), "evidence_id").unwrap(),
        vec![entry.clone(), target],
        NonEmptyString::parse(evidence_text, "evidence_text").unwrap(),
        Some(NonEmptyString::parse("handcrafted-fixture", "source").unwrap()),
        None,
        StringMap::empty(),
    );
    let memory = IngestMemory::new(
        vec![IngestDimension::new(
            dimension.clone(),
            NonEmptyString::parse("agent", "kind").unwrap(),
            Some(NonEmptyString::parse("Writer", "title").unwrap()),
            StringMap::empty(),
        )],
        vec![
            IngestEntry::new(
                entry,
                NonEmptyString::parse("decision", "kind").unwrap(),
                NonEmptyString::parse("Persisted synthetic memory decision.", "text").unwrap(),
                vec![coordinate],
                StringMap::empty(),
            )
            .unwrap(),
        ],
        vec![relation],
        vec![evidence],
    )
    .unwrap();
    ToolArguments::Ingest(IngestArguments::new(
        about_id(about),
        memory,
        None,
        NonEmptyString::parse(format!("idem:{about}:{index}"), "idempotency_key").unwrap(),
        true,
    ))
}

fn temporal_cursor(index: usize) -> TemporalCursor {
    TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse(format!("seq:{index}")).unwrap(),
    )
}

fn about_id(about: &str) -> AboutId {
    AboutId::parse(format!("about:{about}")).unwrap()
}

fn memory_ref(about: &str, label: &str) -> MemoryRef {
    MemoryRef::parse(format!("{about}:node:{label}")).unwrap()
}

fn dimension_ref(label: &str) -> DimensionRef {
    DimensionRef::parse(label).unwrap()
}
