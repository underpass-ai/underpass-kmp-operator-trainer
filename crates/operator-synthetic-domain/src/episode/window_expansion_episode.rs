//! One authored window-expansion episode: a count-over-period question the
//! generator will drive through the multi-step loop to produce SFT
//! trajectories.
//!
//! An episode names the memory it is about (`about`), the question/instruction
//! the policy is given (`goal`, which also conveys the period and the "widen
//! until covered" directive), the expansion parameters (`spec`), and the token
//! budget the session may spend. It deliberately carries no session id — the
//! generator assigns those per run so ids stay unique — and no gold answer: the
//! oracle judges coverage from the kernel's own in-band signals.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;

use crate::episode::window_expansion_spec::WindowExpansionSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowExpansionEpisode {
    about: AboutId,
    goal: TrajectoryGoal,
    spec: WindowExpansionSpec,
    token_budget: u32,
}

impl WindowExpansionEpisode {
    pub fn new(
        about: AboutId,
        goal: TrajectoryGoal,
        spec: WindowExpansionSpec,
        token_budget: u32,
    ) -> Self {
        Self {
            about,
            goal,
            spec,
            token_budget,
        }
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn goal(&self) -> &TrajectoryGoal {
        &self.goal
    }

    pub fn spec(&self) -> WindowExpansionSpec {
        self.spec
    }

    pub fn token_budget(&self) -> u32 {
        self.token_budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    #[test]
    fn exposes_parts() {
        let spec = WindowExpansionSpec::new(
            PositiveCount::parse(8, "window").unwrap(),
            PositiveCount::parse(5, "iterations").unwrap(),
        );
        let episode = WindowExpansionEpisode::new(
            AboutId::parse("about:period").unwrap(),
            TrajectoryGoal::parse("Count workshops across the last four months.").unwrap(),
            spec,
            4096,
        );
        assert_eq!(episode.about().as_str(), "about:period");
        assert_eq!(episode.spec().max_iterations().as_usize(), 5);
        assert_eq!(episode.token_budget(), 4096);
    }
}
