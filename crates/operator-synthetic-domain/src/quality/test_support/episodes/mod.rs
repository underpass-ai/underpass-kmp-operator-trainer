//! Handcrafted v7.2 episodes and their typed trajectory rows.

mod bug_investigation;
mod builders;
mod incident_payments_timeout;
mod product_planning;
mod smart_writing;
mod software_migration;

pub use bug_investigation::episode_bug_investigation;
pub use incident_payments_timeout::episode_incident_payments_timeout;
pub use product_planning::episode_product_planning;
pub use smart_writing::episode_smart_writing;
pub use software_migration::episode_software_migration;

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;
use crate::episode::episode_theme::EpisodeTheme;
use crate::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

use self::bug_investigation::bug_investigation_trajectories;
use self::builders::episode_spec;
use self::incident_payments_timeout::incident_payments_timeout_trajectories;
use self::product_planning::product_planning_trajectories;
use self::smart_writing::smart_writing_trajectories;
use self::software_migration::software_migration_trajectories;

pub fn episode(id: &str) -> SyntheticEpisodeSpec {
    episode_spec(
        id,
        EpisodeTheme::Incident,
        "Resolve the synthetic incident.",
        vec![KmpMcpCapability::Inspect],
    )
}

pub(super) fn seed_episodes() -> Vec<SyntheticEpisodeSpec> {
    vec![
        episode_incident_payments_timeout(),
        episode_software_migration(),
        episode_bug_investigation(),
        episode_product_planning(),
        episode_smart_writing(),
    ]
}

pub(super) fn seed_trajectories() -> Vec<TrainingTrajectory> {
    let mut rows = Vec::new();
    rows.extend(incident_payments_timeout_trajectories());
    rows.extend(software_migration_trajectories());
    rows.extend(bug_investigation_trajectories());
    rows.extend(product_planning_trajectories());
    rows.extend(smart_writing_trajectories());
    rows
}

pub(super) use self::builders::inspect_only_trajectory;

#[cfg(test)]
mod tests {
    use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
    use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

    use super::{
        bug_investigation_trajectories, incident_payments_timeout_trajectories,
        product_planning_trajectories, smart_writing_trajectories, software_migration_trajectories,
    };

    #[test]
    fn episode_incident_payments_timeout_passes_strict_contract() {
        assert_passes_strict_contract(&incident_payments_timeout_trajectories());
    }

    #[test]
    fn episode_software_migration_passes_strict_contract() {
        assert_passes_strict_contract(&software_migration_trajectories());
    }

    #[test]
    fn episode_bug_investigation_passes_strict_contract() {
        assert_passes_strict_contract(&bug_investigation_trajectories());
    }

    #[test]
    fn episode_product_planning_passes_strict_contract() {
        assert_passes_strict_contract(&product_planning_trajectories());
    }

    #[test]
    fn episode_smart_writing_passes_strict_contract() {
        assert_passes_strict_contract(&smart_writing_trajectories());
    }

    fn assert_passes_strict_contract(trajectories: &[TrainingTrajectory]) {
        let validator = CompositeActionContractValidator::default_strict();
        for trajectory in trajectories {
            validator
                .validate(
                    trajectory.target_action(),
                    trajectory.about(),
                    trajectory.mode(),
                    trajectory.visible_state(),
                )
                .expect("handcrafted trajectory must pass strict action contract");
        }
    }
}
