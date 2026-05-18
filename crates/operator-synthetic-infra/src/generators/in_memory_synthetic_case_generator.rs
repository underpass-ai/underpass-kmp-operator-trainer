//! `InMemorySyntheticCaseGenerator` produces a fixed canonical
//! trajectory for each `KmpMcpCapability` and clones it N times to
//! satisfy the spec's minimum. It exists so the application use case
//! and downstream contexts (evaluation, replay, training) can be
//! exercised end-to-end without a real teacher model in the loop.
//!
//! Fixtures are deliberately minimal: they pass every shared-domain
//! invariant and every `CompositeActionContractValidator` rule, but
//! they do not reflect realistic operator behaviour. A production
//! generator backed by an LLM teacher will land in a later pass.

use operator_shared_domain::action::operator_action::OperatorAction;
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
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
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
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_application::error::generate_synthetic_case_error::GenerateSyntheticCaseError;
use operator_synthetic_application::ports::synthetic_case_generator::SyntheticCaseGenerator;
use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
use operator_synthetic_domain::case::synthetic_case_spec::SyntheticCaseSpec;

#[derive(Debug, Default)]
pub struct InMemorySyntheticCaseGenerator;

impl InMemorySyntheticCaseGenerator {
    pub fn new() -> Self {
        Self
    }

    fn build_trajectory(
        spec: &SyntheticCaseSpec,
        index: usize,
    ) -> Result<TrainingTrajectory, GenerateSyntheticCaseError> {
        let capability = spec.capability();
        let mode = capability.mode();
        let trajectory_id =
            TrainingTrajectoryId::parse(format!("{}:{:04}", spec.case_id().as_str(), index))
                .map_err(|err| domain_to_err(spec, &err))?;
        let step_id =
            StepId::parse(format!("step:{index:04}")).map_err(|err| domain_to_err(spec, &err))?;
        let about = AboutId::parse(format!("about:{}:{}", capability.name(), index))
            .map_err(|err| domain_to_err(spec, &err))?;
        let task_family = TaskFamily::parse(format!("{}.{}", mode.as_str(), capability.name()))
            .map_err(|err| domain_to_err(spec, &err))?;
        let (visible_state, target_action) = match capability {
            KmpMcpCapability::Wake => fixture_wake(),
            KmpMcpCapability::Ask => fixture_ask(),
            KmpMcpCapability::Near => fixture_near(),
            KmpMcpCapability::Goto => fixture_goto(),
            KmpMcpCapability::Rewind => fixture_rewind(),
            KmpMcpCapability::Forward => fixture_forward(),
            KmpMcpCapability::Trace => fixture_trace(),
            KmpMcpCapability::Inspect => fixture_inspect(),
            KmpMcpCapability::WriteMemory => fixture_write_memory(),
        };
        TrainingTrajectory::new(
            trajectory_id,
            step_id,
            about,
            mode,
            task_family,
            AllowedTools::for_mode(mode),
            visible_state,
            target_action,
        )
        .map_err(|err| domain_to_err(spec, &err))
    }
}

impl SyntheticCaseGenerator for InMemorySyntheticCaseGenerator {
    fn generate(
        &self,
        spec: &SyntheticCaseSpec,
    ) -> Result<Vec<TrainingTrajectory>, GenerateSyntheticCaseError> {
        let count = spec.minimum_examples().as_usize();
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            out.push(Self::build_trajectory(spec, index)?);
        }
        Ok(out)
    }
}

fn domain_to_err(
    spec: &SyntheticCaseSpec,
    err: &operator_shared_domain::error::domain_error::DomainError,
) -> GenerateSyntheticCaseError {
    GenerateSyntheticCaseError::Generator {
        adapter: "in_memory_synthetic_case_generator",
        case_id: spec.case_id().as_str().to_string(),
        message: err.to_string(),
    }
}

fn fixture_wake() -> (VisibleState, OperatorAction) {
    let visible = VisibleState::assemble([], [], None, BudgetSnapshot::bounded(8, 4096));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
        WakeArguments::new(AboutId::parse("about:fixture-wake").expect("static")),
    )));
    (visible, action)
}

fn fixture_ask() -> (VisibleState, OperatorAction) {
    let visible = VisibleState::assemble([], [], None, BudgetSnapshot::bounded(8, 4096));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
        AskArguments::new("fixture ask").expect("static"),
    )));
    (visible, action)
}

fn fixture_near() -> (VisibleState, OperatorAction) {
    let anchor = MemoryRef::parse("node:near").expect("static");
    let dim = DimensionRef::parse("temporal").expect("static");
    let visible = VisibleState::assemble(
        [anchor.clone()],
        [dim.clone()],
        None,
        BudgetSnapshot::bounded(8, 4096),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
        NearArguments::new(
            anchor,
            vec![dim],
            Some(PositiveCount::parse(3, "limit").expect("static")),
        )
        .expect("static"),
    )));
    (visible, action)
}

fn fixture_goto() -> (VisibleState, OperatorAction) {
    let target = MemoryRef::parse("node:goto").expect("static");
    let visible =
        VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::bounded(8, 4096));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
        GotoArguments::new(Cursor::Ref(RefCursor::new(target))),
    )));
    (visible, action)
}

fn fixture_rewind() -> (VisibleState, OperatorAction) {
    let cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse("2026-05-18T00:00:00Z").expect("static"),
    );
    let visible = VisibleState::assemble(
        [],
        [],
        Some(Cursor::Temporal(cursor.clone())),
        BudgetSnapshot::bounded(8, 4096),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Rewind(
        RewindArguments::new(cursor, PositiveCount::parse(2, "window").expect("static")),
    )));
    (visible, action)
}

fn fixture_forward() -> (VisibleState, OperatorAction) {
    let cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse("2026-05-18T00:00:00Z").expect("static"),
    );
    let visible = VisibleState::assemble(
        [],
        [],
        Some(Cursor::Temporal(cursor.clone())),
        BudgetSnapshot::bounded(8, 4096),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
        ForwardArguments::new(cursor, PositiveCount::parse(2, "window").expect("static")),
    )));
    (visible, action)
}

fn fixture_trace() -> (VisibleState, OperatorAction) {
    let from = MemoryRef::parse("node:trace-from").expect("static");
    let to = MemoryRef::parse("node:trace-to").expect("static");
    let visible = VisibleState::assemble(
        [from.clone(), to.clone()],
        [],
        None,
        BudgetSnapshot::bounded(8, 4096),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
        TraceArguments::new(
            from,
            Some(to),
            PositiveCount::parse(1, "page").expect("static"),
        ),
    )));
    (visible, action)
}

fn fixture_inspect() -> (VisibleState, OperatorAction) {
    let target = MemoryRef::parse("node:inspect").expect("static");
    let visible =
        VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::bounded(8, 4096));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    (visible, action)
}

fn fixture_write_memory() -> (VisibleState, OperatorAction) {
    let visible = VisibleState::assemble([], [], None, BudgetSnapshot::bounded(8, 4096));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
        WriteMemoryArguments::new("fixture summary", "fixture body", vec![]).expect("static"),
    )));
    (visible, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
    use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
    use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;

    fn spec(capability: KmpMcpCapability, minimum: usize) -> SyntheticCaseSpec {
        SyntheticCaseSpec::new(
            SyntheticCaseId::parse(format!("case:{}", capability.name())).unwrap(),
            capability,
            PositiveCount::parse(minimum, "min").unwrap(),
        )
    }

    #[test]
    fn generates_the_requested_count_per_spec() {
        let generator = InMemorySyntheticCaseGenerator::new();
        let s = spec(KmpMcpCapability::Inspect, 3);
        let result = generator.generate(&s).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn every_capability_passes_the_action_contract_validator() {
        let generator = InMemorySyntheticCaseGenerator::new();
        let validator = CompositeActionContractValidator::default_strict();
        for capability in KmpMcpCapability::ALL {
            let s = spec(capability, 1);
            let trajectories = generator.generate(&s).unwrap();
            for tr in &trajectories {
                validator
                    .validate(tr.target_action(), tr.mode(), tr.visible_state())
                    .unwrap_or_else(|err| {
                        panic!(
                            "fixture for {capability} violated contract: {:?}",
                            err.as_slice()
                        )
                    });
            }
        }
    }

    #[test]
    fn trajectories_have_unique_ids_within_a_case() {
        let generator = InMemorySyntheticCaseGenerator::new();
        let s = spec(KmpMcpCapability::Wake, 5);
        let trajectories = generator.generate(&s).unwrap();
        let mut ids: Vec<_> = trajectories
            .iter()
            .map(|t| t.id().as_str().to_string())
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
    }
}
