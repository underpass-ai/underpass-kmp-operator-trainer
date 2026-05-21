//! Product planning episode covering trace plus prepared `write_memory`.

use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use super::builders::{
    active_temporal_cursor, ask, dims, episode_spec, inspect, near, read_tool, refs, stop_row,
    trace, wake, write_memory, write_tool,
};

pub fn episode_product_planning() -> SyntheticEpisodeSpec {
    episode_spec(
        "episode_product_planning",
        EpisodeTheme::ProductDecision,
        "Preserve why the final product plan superseded the earlier option.",
        vec![
            KmpMcpCapability::Wake,
            KmpMcpCapability::Ask,
            KmpMcpCapability::Near,
            KmpMcpCapability::Trace,
            KmpMcpCapability::Inspect,
            KmpMcpCapability::Ask,
            KmpMcpCapability::WriteMemory,
        ],
    )
}

pub(super) fn product_planning_trajectories() -> Vec<TrainingTrajectory> {
    let about = "episode_product_planning";
    let refs = refs(
        about,
        &["request", "constraint", "option_a", "option_b", "decision"],
    );
    let dims = dims(&["agent:pm", "agent:reviewer"]);
    vec![
        read_tool(
            about,
            1,
            "product.wake",
            "Wake the product planning memory.",
            refs.clone(),
            dims.clone(),
            None,
            6,
            wake(about),
        ),
        read_tool(
            about,
            2,
            "product.ask",
            "Ask for active product constraints.",
            refs.clone(),
            dims.clone(),
            None,
            5,
            ask("Which product constraints are active?"),
        ),
        read_tool(
            about,
            3,
            "product.near",
            "Expand around option B.",
            refs.clone(),
            dims.clone(),
            None,
            4,
            near(refs[3].clone(), dims[0].clone()),
        ),
        read_tool(
            about,
            4,
            "product.trace",
            "Trace why option B superseded option A.",
            refs.clone(),
            dims.clone(),
            Some(active_temporal_cursor(32)),
            3,
            trace(refs[2].clone(), refs[3].clone()),
        ),
        read_tool(
            about,
            5,
            "product.inspect",
            "Inspect the final decision.",
            refs.clone(),
            dims.clone(),
            None,
            2,
            inspect(refs[4].clone()),
        ),
        read_tool(
            about,
            6,
            "product.ask",
            "Ask whether the prepared write has evidence.",
            refs.clone(),
            dims.clone(),
            None,
            1,
            ask("Is the prepared product decision write proven?"),
        ),
        write_tool(
            about,
            7,
            "product.write_memory",
            "Execute the prepared product memory write.",
            refs.clone(),
            dims.clone(),
            write_memory(
                "Product decision accepted.",
                "Option B supersedes option A because it satisfies the active constraint.",
                vec![refs[3].clone(), refs[4].clone()],
            ),
        ),
        stop_row(
            about,
            8,
            OperatorMode::Read,
            "product.stop",
            "Stop after the product decision is recorded.",
            refs.clone(),
            dims.clone(),
            StopReason::AnswerReady,
            Some("The product plan has evidence and a recorded decision."),
            vec![refs[4].clone()],
            0,
        ),
    ]
}
