//! Maps an authored window-expansion episode DTO to its domain form.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_synthetic_domain::episode::window_expansion_episode::WindowExpansionEpisode;
use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;
use operator_synthetic_domain::error::synthetic_domain_result::SyntheticDomainResult;

use crate::dto::window_expansion_episode_dto::WindowExpansionEpisodeDto;

#[derive(Debug)]
pub struct WindowExpansionEpisodeMapper;

impl WindowExpansionEpisodeMapper {
    pub fn to_domain(
        dto: &WindowExpansionEpisodeDto,
    ) -> SyntheticDomainResult<WindowExpansionEpisode> {
        let about = AboutId::parse(dto.about.clone())?;
        let goal = TrajectoryGoal::parse(dto.goal.clone())?;
        let initial_window = PositiveCount::parse(
            dto.initial_window,
            "window_expansion_episode.initial_window",
        )?;
        let max_iterations = PositiveCount::parse(
            dto.max_iterations,
            "window_expansion_episode.max_iterations",
        )?;
        let spec = WindowExpansionSpec::new(initial_window, max_iterations);
        let mut expected_refs = Vec::with_capacity(dto.expected_refs.len());
        for raw in &dto.expected_refs {
            expected_refs.push(MemoryRef::parse(raw.clone())?);
        }
        Ok(
            WindowExpansionEpisode::new(about, goal, spec, dto.token_budget)
                .with_expected_refs(expected_refs),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_well_formed_episode_with_gold() {
        let dto = WindowExpansionEpisodeDto {
            about: "about:period".to_string(),
            goal: "Count workshops across the last four months.".to_string(),
            initial_window: 8,
            max_iterations: 5,
            token_budget: 4096,
            expected_refs: vec!["node:1".to_string(), "node:2".to_string()],
        };
        let episode = WindowExpansionEpisodeMapper::to_domain(&dto).expect("maps");
        assert_eq!(episode.about().as_str(), "about:period");
        assert_eq!(episode.spec().initial_window().as_usize(), 8);
        assert_eq!(episode.token_budget(), 4096);
        assert!(episode.has_gold());
        assert_eq!(episode.expected_refs().len(), 2);
    }

    #[test]
    fn rejects_a_zero_window() {
        let dto = WindowExpansionEpisodeDto {
            about: "about:period".to_string(),
            goal: "Count.".to_string(),
            initial_window: 0,
            max_iterations: 5,
            token_budget: 4096,
            expected_refs: vec![],
        };
        assert!(WindowExpansionEpisodeMapper::to_domain(&dto).is_err());
    }

    #[test]
    fn rejects_a_malformed_gold_ref() {
        let dto = WindowExpansionEpisodeDto {
            about: "about:period".to_string(),
            goal: "Count.".to_string(),
            initial_window: 4,
            max_iterations: 3,
            token_budget: 4096,
            expected_refs: vec![String::new()],
        };
        assert!(WindowExpansionEpisodeMapper::to_domain(&dto).is_err());
    }
}
