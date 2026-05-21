pub mod composite_corpus_quality_validator;
pub mod corpus_audit_snapshot;
pub mod corpus_quality_validator;
pub mod corpus_quality_violations;
pub mod corpus_snapshot;
pub mod episode_split_snapshot;

#[cfg(test)]
pub(crate) mod test_support {
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::cursor::cursor::Cursor;
    use operator_shared_domain::cursor::ref_cursor::RefCursor;
    use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
    use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
    use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::dataset_id::DatasetId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
    use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
    use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
    use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
    use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
    use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
    use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
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
    use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::string_map::StringMap;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    use crate::capability::kmp_mcp_capability::KmpMcpCapability;
    use crate::dataset::synthetic_dataset::SyntheticDataset;
    use crate::episode::capability_target::CapabilityTarget;
    use crate::episode::episode_id::EpisodeId;
    use crate::episode::episode_objective::EpisodeObjective;
    use crate::episode::episode_step_plan::EpisodeStepPlan;
    use crate::episode::episode_theme::EpisodeTheme;
    use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;
    use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
    use crate::quality::corpus_snapshot::CorpusSnapshot;
    use crate::quality::episode_split_snapshot::EpisodeSplitSnapshot;

    pub(crate) fn episode(id: &str) -> SyntheticEpisodeSpec {
        SyntheticEpisodeSpec::new(
            EpisodeId::parse(id).unwrap(),
            EpisodeTheme::Incident,
            EpisodeObjective::parse("Resolve the synthetic incident.").unwrap(),
            vec![EpisodeStepPlan::new(
                CapabilityTarget::new(
                    KmpMcpCapability::Inspect,
                    PositiveCount::parse(1, "minimum").unwrap(),
                ),
                EpisodeObjective::parse("Inspect evidence.").unwrap(),
            )],
        )
        .unwrap()
    }

    pub(crate) fn inspect_dataset() -> SyntheticDataset {
        SyntheticDataset::new(
            DatasetId::parse("dataset:inspect").unwrap(),
            vec![trajectory(KmpMcpCapability::Inspect, 1, "step:inspect")],
        )
        .unwrap()
    }

    pub(crate) fn snapshot_with_audit(audit: CorpusAuditSnapshot) -> CorpusSnapshot {
        CorpusSnapshot::new(
            all_tool_dataset(),
            vec![episode("episode:train"), episode("episode:eval")],
            audit,
            Some(
                EpisodeSplitSnapshot::new(
                    vec![EpisodeId::parse("episode:train").unwrap()],
                    vec![EpisodeId::parse("episode:eval").unwrap()],
                )
                .unwrap(),
            ),
        )
        .unwrap()
    }

    pub(crate) fn full_quality_snapshot() -> CorpusSnapshot {
        snapshot_with_audit(CorpusAuditSnapshot::clean())
    }

    pub(crate) fn inspect_snapshot_without_split() -> CorpusSnapshot {
        CorpusSnapshot::new(
            inspect_dataset(),
            vec![episode("episode:inspect")],
            CorpusAuditSnapshot::clean(),
            None,
        )
        .unwrap()
    }

    pub(crate) fn duplicate_step_snapshot() -> CorpusSnapshot {
        CorpusSnapshot::new(
            SyntheticDataset::new(
                DatasetId::parse("dataset:duplicates").unwrap(),
                vec![
                    trajectory(KmpMcpCapability::Inspect, 1, "step:dup"),
                    trajectory(KmpMcpCapability::Inspect, 2, "step:dup"),
                ],
            )
            .unwrap(),
            vec![episode("episode:duplicates")],
            CorpusAuditSnapshot::clean(),
            None,
        )
        .unwrap()
    }

    fn all_tool_dataset() -> SyntheticDataset {
        let trajectories = KmpMcpCapability::ALL
            .iter()
            .copied()
            .enumerate()
            .map(|(index, capability)| {
                trajectory(
                    capability,
                    index + 1,
                    &format!("step:{}", capability.name()),
                )
            })
            .collect();
        SyntheticDataset::new(DatasetId::parse("dataset:all-tools").unwrap(), trajectories).unwrap()
    }

    fn trajectory(capability: KmpMcpCapability, index: usize, step_id: &str) -> TrainingTrajectory {
        let about = AboutId::parse(format!("about:{}", capability.name())).unwrap();
        let target = MemoryRef::parse(format!("node:{}", capability.name())).unwrap();
        let other = MemoryRef::parse(format!("node:{}:other", capability.name())).unwrap();
        let dimension = DimensionRef::parse("agent:operator").unwrap();
        let visible = VisibleState::assemble(
            [target.clone(), other.clone()],
            [dimension.clone()],
            None,
            BudgetSnapshot::bounded(8, 4096),
        );
        let action = OperatorAction::ToolCall(ToolCallAction::new(tool_args(
            capability,
            about.clone(),
            target,
            other,
            dimension,
            index,
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(format!("trajectory:{}:{index}", capability.name()))
                .unwrap(),
            StepId::parse(step_id).unwrap(),
            about,
            capability.mode(),
            TaskFamily::parse(format!(
                "{}.{}",
                capability.mode().as_str(),
                capability.name()
            ))
            .unwrap(),
            TrajectoryGoal::parse(format!("Use {} correctly.", capability.tool().as_str()))
                .unwrap(),
            AllowedTools::for_mode(capability.mode()),
            visible,
            action,
        )
        .unwrap()
    }

    fn tool_args(
        capability: KmpMcpCapability,
        about: AboutId,
        target: MemoryRef,
        other: MemoryRef,
        dimension: DimensionRef,
        index: usize,
    ) -> ToolArguments {
        match capability {
            KmpMcpCapability::Ingest => ToolArguments::Ingest(ingest_args(about, dimension, index)),
            KmpMcpCapability::Wake => ToolArguments::Wake(WakeArguments::new(about)),
            KmpMcpCapability::Ask => {
                ToolArguments::Ask(AskArguments::new("What is known now?").unwrap())
            }
            KmpMcpCapability::Near => ToolArguments::Near(
                NearArguments::new(
                    target,
                    vec![dimension],
                    Some(PositiveCount::parse(3, "limit").unwrap()),
                )
                .unwrap(),
            ),
            KmpMcpCapability::Goto => {
                ToolArguments::Goto(GotoArguments::new(Cursor::Ref(RefCursor::new(target))))
            }
            KmpMcpCapability::Rewind => ToolArguments::Rewind(RewindArguments::new(
                temporal_cursor(index),
                PositiveCount::parse(2, "window").unwrap(),
            )),
            KmpMcpCapability::Forward => ToolArguments::Forward(ForwardArguments::new(
                temporal_cursor(index),
                PositiveCount::parse(2, "window").unwrap(),
            )),
            KmpMcpCapability::Trace => ToolArguments::Trace(TraceArguments::new(
                target,
                Some(other),
                PositiveCount::parse(8, "page").unwrap(),
            )),
            KmpMcpCapability::Inspect => ToolArguments::Inspect(InspectArguments::new(target)),
            KmpMcpCapability::WriteMemory => ToolArguments::WriteMemory(
                WriteMemoryArguments::new("summary", "body", vec![target]).unwrap(),
            ),
        }
    }

    fn temporal_cursor(index: usize) -> TemporalCursor {
        TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse(format!("seq:{index}")).unwrap(),
        )
    }

    fn ingest_args(about: AboutId, dimension: DimensionRef, index: usize) -> IngestArguments {
        let entry = MemoryRef::parse(format!("node:ingest:{index}")).unwrap();
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
        let memory = IngestMemory::new(
            vec![IngestDimension::new(
                dimension,
                NonEmptyString::parse("agent", "kind").unwrap(),
                Some(NonEmptyString::parse("Operator", "title").unwrap()),
                StringMap::empty(),
            )],
            vec![
                IngestEntry::new(
                    entry,
                    NonEmptyString::parse("observation", "kind").unwrap(),
                    NonEmptyString::parse("Synthetic observation.", "text").unwrap(),
                    vec![coordinate],
                    StringMap::empty(),
                )
                .unwrap(),
            ],
            vec![],
            vec![],
        )
        .unwrap();
        IngestArguments::new(
            about,
            memory,
            None,
            NonEmptyString::parse(format!("idem:{index}"), "idempotency_key").unwrap(),
            true,
        )
    }
}
