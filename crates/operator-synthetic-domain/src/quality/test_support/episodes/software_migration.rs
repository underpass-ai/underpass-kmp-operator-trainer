//! Software migration episode covering forward movement and canonical ingest.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use super::builders::{
    active_temporal_cursor, anemic_ingest, ask, dims, episode_spec, forward, inspect, near,
    read_tool, refs, stop_row, wake, write_tool,
};

pub fn episode_software_migration() -> SyntheticEpisodeSpec {
    episode_spec(
        "episode_software_migration",
        EpisodeTheme::Migration,
        "Track a migration decision from stale plan to accepted state.",
        vec![
            KmpMcpCapability::Wake,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Forward,
            KmpMcpCapability::Inspect,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Ingest,
        ],
    )
}

#[allow(clippy::too_many_lines)]
pub(super) fn software_migration_trajectories() -> Vec<TrainingTrajectory> {
    let about = "episode_software_migration";
    let refs = refs(about, &["plan", "constraint", "stale", "accepted", "note"]);
    let dims = dims(&["agent:migration", "session:planning"]);
    vec![
        read_tool(
            about,
            1,
            "migration.wake",
            "Wake the migration memory.",
            refs.clone(),
            dims.clone(),
            None,
            6,
            wake(about),
        ),
        read_tool(
            about,
            2,
            "migration.ask",
            "Ask for migration constraints.",
            refs.clone(),
            dims.clone(),
            None,
            5,
            ask("Which constraints are still active?"),
        ),
        read_tool(
            about,
            3,
            "migration.near",
            "Expand around the stale plan.",
            refs.clone(),
            dims.clone(),
            None,
            4,
            near(refs[2].clone(), dims[0].clone()),
        ),
        read_tool(
            about,
            4,
            "migration.forward",
            "Move forward to the accepted migration state.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(4)),
            3,
            forward(4),
        ),
        read_tool(
            about,
            5,
            "migration.inspect",
            "Inspect the accepted state.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            inspect(refs[3].clone()),
        ),
        read_tool(
            about,
            6,
            "migration.ask",
            "Ask whether the migration note is ready.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            ask("Is the migration note ready to persist?"),
        ),
        read_tool(
            about,
            7,
            "migration.near",
            "Expand around the accepted migration state.",
            refs.clone(),
            dims.clone(),
            None,
            1,
            near(refs[3].clone(), dims[1].clone()),
        ),
        write_tool(
            about,
            8,
            "migration.ingest",
            "Execute canonical ingest with an anemic fallback relation.",
            refs.clone(),
            dims.clone(),
            anemic_ingest(about, refs[3].clone(), &dims[0], 8),
        ),
        stop_row(
            about,
            9,
            OperatorMode::Read,
            "migration.stop",
            "Stop after the migration write is prepared.",
            refs.clone(),
            dims.clone(),
            StopReason::AnswerReady,
            Some("Migration state was persisted with explicit fallback relation."),
            vec![refs[3].clone()],
            0,
        ),
    ]
}
