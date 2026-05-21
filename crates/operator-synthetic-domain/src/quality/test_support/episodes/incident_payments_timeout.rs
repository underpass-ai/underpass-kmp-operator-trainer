//! Technical incident episode covering read traversal and answer-ready stop.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use super::builders::{
    active_temporal_cursor, ask, dims, episode_spec, inspect, near, read_tool, refs, rewind,
    stop_row, trace, wake,
};

pub fn episode_incident_payments_timeout() -> SyntheticEpisodeSpec {
    episode_spec(
        "episode_incident_payments_timeout",
        EpisodeTheme::Incident,
        "Resolve a payments timeout incident without losing the failed path.",
        vec![
            KmpMcpCapability::Wake,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Trace,
            KmpMcpCapability::Inspect,
            KmpMcpCapability::Rewind,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Inspect,
        ],
    )
}

pub(super) fn incident_payments_timeout_trajectories() -> Vec<TrainingTrajectory> {
    let about = "episode_incident_payments_timeout";
    let refs = refs(about, &["alert", "triage", "worker", "rollback", "fix"]);
    let dims = dims(&["agent:triage", "agent:solver"]);
    vec![
        read_tool(
            about,
            1,
            "incident.wake",
            "Wake the payment incident memory.",
            refs.clone(),
            dims.clone(),
            None,
            6,
            wake(about),
        ),
        read_tool(
            about,
            2,
            "incident.ask",
            "Ask for deterministic timeout evidence.",
            refs.clone(),
            dims.clone(),
            None,
            5,
            ask("What timeout evidence is visible?"),
        ),
        read_tool(
            about,
            3,
            "incident.near",
            "Expand around the triage signal.",
            refs.clone(),
            dims.clone(),
            None,
            4,
            near(refs[1].clone(), dims[0].clone()),
        ),
        read_tool(
            about,
            4,
            "incident.trace",
            "Trace why the fix replaced rollback.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(16)),
            3,
            trace(refs[3].clone(), refs[4].clone()),
        ),
        read_tool(
            about,
            5,
            "incident.inspect",
            "Inspect the fix evidence.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            inspect(refs[4].clone()),
        ),
        read_tool(
            about,
            6,
            "incident.rewind",
            "Rewind before the rollback assumption.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(6)),
            1,
            rewind(6),
        ),
        read_tool(
            about,
            7,
            "incident.ask",
            "Ask whether enough evidence is present.",
            refs.clone(),
            dims.clone(),
            None,
            1,
            ask("Is the timeout cause proven?"),
        ),
        stop_row(
            about,
            8,
            OperatorMode::Read,
            "incident.stop",
            "Stop when incident evidence is sufficient.",
            refs.clone(),
            dims.clone(),
            StopReason::AnswerReady,
            Some("Timeout root cause is proven by fix evidence."),
            vec![refs[4].clone()],
            0,
        ),
    ]
}
