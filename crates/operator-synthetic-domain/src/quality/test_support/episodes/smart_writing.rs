//! Smart writing episode covering read-before-write proof and rich ingest.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use super::builders::{
    ask, dims, episode_spec, inspect, near, read_tool, refs, rich_ingest, stop_row, wake,
    write_tool,
};

pub fn episode_smart_writing() -> SyntheticEpisodeSpec {
    episode_spec(
        "episode_smart_writing",
        EpisodeTheme::SmartWritingSession,
        "Write a new memory entry only after proving the relation target.",
        vec![
            KmpMcpCapability::Wake,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Inspect,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Ingest,
        ],
    )
}

pub(super) fn smart_writing_trajectories() -> Vec<TrainingTrajectory> {
    let about = "episode_smart_writing";
    let refs = refs(
        about,
        &["question", "prior", "evidence", "candidate", "decision"],
    );
    let dims = dims(&["agent:writer", "session:smart-write"]);
    vec![
        read_tool(
            about,
            1,
            "smart.wake",
            "Wake smart writing context.",
            refs.clone(),
            dims.clone(),
            None,
            6,
            wake(about),
        ),
        read_tool(
            about,
            2,
            "smart.ask",
            "Ask which prior node can support a relation.",
            refs.clone(),
            dims.clone(),
            None,
            5,
            ask("Which prior node supports the new memory?"),
        ),
        read_tool(
            about,
            3,
            "smart.near",
            "Expand around the prior node.",
            refs.clone(),
            dims.clone(),
            None,
            4,
            near(refs[1].clone(), dims[0].clone()),
        ),
        read_tool(
            about,
            4,
            "smart.inspect",
            "Inspect read-before-write proof.",
            refs.clone(),
            dims.clone(),
            None,
            3,
            inspect(refs[2].clone()),
        ),
        read_tool(
            about,
            5,
            "smart.ask",
            "Ask whether relation evidence is sufficient.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            ask("Can the relation be justified honestly?"),
        ),
        read_tool(
            about,
            6,
            "smart.near",
            "Expand around the candidate write target.",
            refs.clone(),
            dims.clone(),
            None,
            1,
            near(refs[3].clone(), dims[1].clone()),
        ),
        write_tool(
            about,
            7,
            "smart.ingest",
            "Execute rich ingest with why and evidence.",
            refs.clone(),
            dims.clone(),
            rich_ingest(about, refs[2].clone(), &dims[0], 7),
        ),
        stop_row(
            about,
            8,
            OperatorMode::Write,
            "smart.stop",
            "Stop after rich memory is written.",
            refs.clone(),
            dims.clone(),
            StopReason::AnswerReady,
            Some("The rich relation was written with explicit evidence."),
            vec![refs[2].clone()],
            0,
        ),
    ]
}
