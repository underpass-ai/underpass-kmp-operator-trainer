//! `CorpusSnapshot` fixtures assembled from the v7.2 seed episodes.

use operator_shared_domain::ids::dataset_id::DatasetId;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::value_objects::example_count::ExampleCount;

use crate::dataset::synthetic_dataset::SyntheticDataset;
use crate::episode::episode_id::EpisodeId;
use crate::quality::corpus_audit_snapshot::CorpusAuditSnapshot;
use crate::quality::corpus_snapshot::CorpusSnapshot;
use crate::quality::episode_split_snapshot::EpisodeSplitSnapshot;
use crate::quality::test_support::episodes::{
    episode, inspect_only_trajectory, seed_episodes, seed_trajectories,
};

pub fn clean_corpus_snapshot() -> CorpusSnapshot {
    CorpusSnapshot::new(
        seed_dataset(),
        seed_episodes(),
        CorpusAuditSnapshot::clean(),
        Some(seed_split()),
    )
    .unwrap()
}

pub fn full_quality_snapshot() -> CorpusSnapshot {
    clean_corpus_snapshot()
}

pub fn snapshot_with_audit(audit: CorpusAuditSnapshot) -> CorpusSnapshot {
    CorpusSnapshot::new(seed_dataset(), seed_episodes(), audit, Some(seed_split())).unwrap()
}

pub fn inspect_dataset() -> SyntheticDataset {
    SyntheticDataset::new(
        DatasetId::parse("dataset:inspect").unwrap(),
        vec![inspect_only_trajectory(
            "episode:inspect",
            1,
            "step:inspect",
        )],
    )
    .unwrap()
}

pub fn inspect_snapshot_without_split() -> CorpusSnapshot {
    CorpusSnapshot::new(
        inspect_dataset(),
        vec![episode("episode:inspect")],
        CorpusAuditSnapshot::clean(),
        None,
    )
    .unwrap()
}

pub fn duplicate_step_snapshot() -> CorpusSnapshot {
    corpus_with_duplicate_model_facing_rows()
}

pub fn corpus_with_unparseable_row() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_schema_parse_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_invalid_action_target() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_action_parse_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_missing_kernel_forward() -> CorpusSnapshot {
    let trajectories = seed_trajectories()
        .into_iter()
        .filter(|trajectory| trajectory.target_action().tool() != Some(KernelTool::Forward))
        .collect();
    CorpusSnapshot::new(
        SyntheticDataset::new(
            DatasetId::parse("dataset:v7:missing-forward").unwrap(),
            trajectories,
        )
        .unwrap(),
        seed_episodes(),
        CorpusAuditSnapshot::clean(),
        Some(seed_split()),
    )
    .unwrap()
}

pub fn corpus_with_ingest_in_read_mode() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_mode_safety_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_unknown_memory_ref() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_reference_safety_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_about_not_in_known() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_scope_safety_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_trace_lacking_cursor_continuation() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_pagination_safety_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_write_lacking_read_before_write() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_write_proof_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_target_action_leaked_in_system_prompt() -> CorpusSnapshot {
    snapshot_with_audit(CorpusAuditSnapshot::clean().with_gold_leak_findings(ExampleCount::new(1)))
}

pub fn corpus_with_train_and_eval_sharing_about() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean().with_episode_split_failures(ExampleCount::new(1)),
    )
}

pub fn corpus_with_duplicate_model_facing_rows() -> CorpusSnapshot {
    let mut trajectories = seed_trajectories();
    let duplicate = trajectories.first().unwrap().clone();
    trajectories.push(duplicate);
    CorpusSnapshot::new(
        SyntheticDataset::new(
            DatasetId::parse("dataset:v7:duplicate-step").unwrap(),
            trajectories,
        )
        .unwrap(),
        seed_episodes(),
        CorpusAuditSnapshot::clean(),
        Some(seed_split()),
    )
    .unwrap()
}

pub fn corpus_with_action_failing_mcp_request_shape() -> CorpusSnapshot {
    snapshot_with_audit(CorpusAuditSnapshot::clean().with_replay_failures(ExampleCount::new(1)))
}

pub fn corpus_without_frontier_baseline_recorded() -> CorpusSnapshot {
    snapshot_with_audit(CorpusAuditSnapshot::clean().without_frontier_ceiling())
}

pub fn corpus_failing_five_specs() -> CorpusSnapshot {
    snapshot_with_audit(
        CorpusAuditSnapshot::clean()
            .with_reference_safety_failures(ExampleCount::new(1))
            .with_mode_safety_failures(ExampleCount::new(1))
            .with_write_proof_failures(ExampleCount::new(1))
            .with_gold_leak_findings(ExampleCount::new(1))
            .with_episode_split_failures(ExampleCount::new(1)),
    )
}

fn seed_split() -> EpisodeSplitSnapshot {
    EpisodeSplitSnapshot::new(
        vec![
            EpisodeId::parse("episode_incident_payments_timeout").unwrap(),
            EpisodeId::parse("episode_software_migration").unwrap(),
            EpisodeId::parse("episode_bug_investigation").unwrap(),
        ],
        vec![
            EpisodeId::parse("episode_product_planning").unwrap(),
            EpisodeId::parse("episode_smart_writing").unwrap(),
        ],
    )
    .unwrap()
}

fn seed_dataset() -> SyntheticDataset {
    SyntheticDataset::new(
        DatasetId::parse("dataset:v7:seed-episodes").unwrap(),
        seed_trajectories(),
    )
    .unwrap()
}
