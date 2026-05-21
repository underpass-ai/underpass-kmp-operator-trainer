//! Bug investigation episode covering goto, rewind, and one escalation.

use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use super::builders::{
    active_temporal_cursor, ask, dims, episode_spec, escalate_row, goto, inspect, read_tool, refs,
    rewind, trace, wake,
};

pub fn episode_bug_investigation() -> SyntheticEpisodeSpec {
    episode_spec(
        "episode_bug_investigation",
        EpisodeTheme::BugInvestigation,
        "Investigate a nondeterministic login bug and escalate only once.",
        vec![
            KmpMcpCapability::Wake,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Trace,
            KmpMcpCapability::Inspect,
            KmpMcpCapability::Goto,
            KmpMcpCapability::Rewind,
            KmpMcpCapability::Ask,
        ],
    )
}

pub(super) fn bug_investigation_trajectories() -> Vec<TrainingTrajectory> {
    let about = "episode_bug_investigation";
    let refs = refs(about, &["report", "logs", "hypothesis", "repro", "unknown"]);
    let dims = dims(&["agent:debugger", "session:nightly"]);
    vec![
        read_tool(
            about,
            1,
            "bug.wake",
            "Wake the bug investigation memory.",
            refs.clone(),
            dims.clone(),
            None,
            6,
            wake(about),
        ),
        read_tool(
            about,
            2,
            "bug.ask",
            "Ask for reproducibility evidence.",
            refs.clone(),
            dims.clone(),
            None,
            5,
            ask("What evidence reproduces the bug?"),
        ),
        read_tool(
            about,
            3,
            "bug.trace",
            "Trace why the first hypothesis failed.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(24)),
            4,
            trace(refs[2].clone(), refs[3].clone()),
        ),
        read_tool(
            about,
            4,
            "bug.inspect",
            "Inspect the repro evidence.",
            refs.clone(),
            dims.clone(),
            None,
            3,
            inspect(refs[3].clone()),
        ),
        read_tool(
            about,
            5,
            "bug.goto",
            "Go to the original bug report.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            goto(refs[0].clone()),
        ),
        read_tool(
            about,
            6,
            "bug.rewind",
            "Rewind to the failed hypothesis.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(6)),
            1,
            rewind(6),
        ),
        read_tool(
            about,
            7,
            "bug.ask",
            "Ask whether Operator can decide the root cause.",
            refs.clone(),
            dims.clone(),
            None,
            1,
            ask("Is the root cause decidable without code reasoning?"),
        ),
        escalate_row(
            about,
            8,
            "bug.escalate",
            "Escalate because deciding the root cause needs a larger model.",
            refs.clone(),
            dims.clone(),
            EscalateReason::BeyondCapability,
            0,
        ),
    ]
}
