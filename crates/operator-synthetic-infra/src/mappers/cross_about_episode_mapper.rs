//! Maps an authored cross-about count episode DTO to its domain form.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;
use operator_synthetic_domain::episode::cross_about_target::CrossAboutTarget;
use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;
use operator_synthetic_domain::error::synthetic_domain_error::SyntheticDomainError;
use operator_synthetic_domain::error::synthetic_domain_result::SyntheticDomainResult;

use crate::dto::cross_about_episode_dto::CrossAboutEpisodeDto;

#[derive(Debug)]
pub struct CrossAboutEpisodeMapper;

impl CrossAboutEpisodeMapper {
    pub fn to_domain(dto: &CrossAboutEpisodeDto) -> SyntheticDomainResult<CrossAboutEpisode> {
        if dto.targets.is_empty() {
            return Err(SyntheticDomainError::EmptyCrossAboutTargets);
        }
        let goal = TrajectoryGoal::parse(dto.goal.clone())?;
        let initial_window =
            PositiveCount::parse(dto.initial_window, "cross_about_episode.initial_window")?;
        let max_iterations =
            PositiveCount::parse(dto.max_iterations, "cross_about_episode.max_iterations")?;
        let spec = WindowExpansionSpec::new(initial_window, max_iterations);

        let mut targets = Vec::with_capacity(dto.targets.len());
        for target_dto in &dto.targets {
            let about = AboutId::parse(target_dto.about.clone())?;
            if target_dto.expected_refs.is_empty() {
                return Err(SyntheticDomainError::CrossAboutTargetMissingGold {
                    about: about.as_str().to_string(),
                });
            }
            let mut expected_refs = Vec::with_capacity(target_dto.expected_refs.len());
            for raw in &target_dto.expected_refs {
                expected_refs.push(MemoryRef::parse(raw.clone())?);
            }
            targets.push(CrossAboutTarget::new(about, expected_refs));
        }

        Ok(CrossAboutEpisode::new(
            targets,
            goal,
            spec,
            dto.token_budget,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::cross_about_target_dto::CrossAboutTargetDto;

    fn target(about: &str, refs: &[&str]) -> CrossAboutTargetDto {
        CrossAboutTargetDto {
            about: about.to_string(),
            expected_refs: refs.iter().map(|r| (*r).to_string()).collect(),
        }
    }

    fn dto(targets: Vec<CrossAboutTargetDto>) -> CrossAboutEpisodeDto {
        CrossAboutEpisodeDto {
            targets,
            goal: "Count workshops across EU and US.".to_string(),
            initial_window: 4,
            max_iterations: 8,
            token_budget: 8192,
        }
    }

    #[test]
    fn maps_a_well_formed_cross_about_episode() {
        let episode = CrossAboutEpisodeMapper::to_domain(&dto(vec![
            target("ctrl-ws-eu", &["ctrl-ws-eu:wkshop-01"]),
            target("ctrl-ws-us", &["ctrl-ws-us:wkshop-02"]),
        ]))
        .expect("maps");
        assert_eq!(episode.entry_about().as_str(), "ctrl-ws-eu");
        assert_eq!(episode.targets().len(), 2);
        assert_eq!(episode.expected_refs().len(), 2);
        assert!(episode.has_gold());
    }

    #[test]
    fn rejects_empty_targets() {
        let err = CrossAboutEpisodeMapper::to_domain(&dto(vec![])).unwrap_err();
        assert!(matches!(err, SyntheticDomainError::EmptyCrossAboutTargets));
    }

    #[test]
    fn rejects_a_target_without_gold() {
        let err =
            CrossAboutEpisodeMapper::to_domain(&dto(vec![target("ctrl-ws-eu", &[])])).unwrap_err();
        assert!(matches!(
            err,
            SyntheticDomainError::CrossAboutTargetMissingGold { .. }
        ));
    }

    #[test]
    fn rejects_a_malformed_gold_ref() {
        let err = CrossAboutEpisodeMapper::to_domain(&dto(vec![target("ctrl-ws-eu", &[""])]))
            .unwrap_err();
        assert!(matches!(err, SyntheticDomainError::Shared(_)));
    }
}
