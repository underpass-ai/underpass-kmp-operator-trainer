//! Compile a cross-about count episode into a runtime [`OperatorRequest`].
//!
//! The synthetic-to-runtime bridge for the cross-about generator. It enters at
//! the episode's first target about and sizes the call budget so the policy can
//! wake into every target about and widen each one's window: one wake plus
//! `max_iterations` expansion calls per about, plus one final terminal decision.
//!
//! Like the single-about compiler it starts from an empty visible state — the
//! operator knows no refs yet and must wake the entry about first — and runs a
//! read session over the canonical read tools (which include `kernel_wake`, the
//! action that switches between abouts). The set of abouts to visit is conveyed
//! by the `goal`, not whitelisted in the request, so the enumeration stays the
//! policy's job and no new about-scoping invariant is introduced.

use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::error::runtime_domain_result::RuntimeDomainResult;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;

#[derive(Debug)]
pub struct CrossAboutEpisodeCompiler;

impl CrossAboutEpisodeCompiler {
    pub fn compile(
        session_id: OperatorSessionId,
        episode: &CrossAboutEpisode,
    ) -> RuntimeDomainResult<OperatorRequest> {
        let budget = SessionBudget::new(required_calls(episode), episode.token_budget());
        // Seed the candidate sessions retrieval surfaced — the abouts the
        // operator must cover — so the entry state carries the session set
        // (anonymized consistently downstream), making cross-about enumeration
        // learnable rather than a guess.
        let candidate_abouts: Vec<AboutId> = episode
            .targets()
            .iter()
            .map(|target| target.about().clone())
            .collect();
        let initial_state = VisibleState::assemble([], [], None, budget_snapshot(budget))
            .with_candidate_abouts(candidate_abouts);
        OperatorRequest::new(
            session_id,
            episode.goal().clone(),
            initial_state,
            OperatorMode::Read,
            AllowedTools::for_mode(OperatorMode::Read),
            budget,
            episode.entry_about().clone(),
            None,
        )
    }
}

/// One wake plus `max_iterations` expansion calls per target about, plus one
/// terminal decision. Saturates rather than overflowing.
fn required_calls(episode: &CrossAboutEpisode) -> u32 {
    let max_iterations =
        u32::try_from(episode.spec().max_iterations().as_usize()).unwrap_or(u32::MAX);
    let targets = u32::try_from(episode.targets().len()).unwrap_or(u32::MAX);
    targets
        .saturating_mul(max_iterations.saturating_add(1))
        .saturating_add(1)
}

fn budget_snapshot(budget: SessionBudget) -> BudgetSnapshot {
    BudgetSnapshot::bounded(
        budget.calls_remaining() as usize,
        budget.tokens_remaining() as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_synthetic_domain::episode::cross_about_target::CrossAboutTarget;
    use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;

    fn episode(num_targets: usize, max_iterations: usize) -> CrossAboutEpisode {
        let targets = (0..num_targets)
            .map(|i| {
                CrossAboutTarget::new(
                    AboutId::parse(format!("about:{i}")).unwrap(),
                    vec![MemoryRef::parse(format!("node:{i}")).unwrap()],
                )
            })
            .collect();
        CrossAboutEpisode::new(
            targets,
            TrajectoryGoal::parse("Count across abouts.").unwrap(),
            WindowExpansionSpec::new(
                PositiveCount::parse(4, "window").unwrap(),
                PositiveCount::parse(max_iterations, "iterations").unwrap(),
            ),
            8192,
        )
    }

    #[test]
    fn enters_at_the_first_about_and_sizes_the_budget_across_abouts() {
        let session_id = OperatorSessionId::parse("xab:0001").unwrap();
        let request = CrossAboutEpisodeCompiler::compile(session_id, &episode(3, 5)).unwrap();
        assert_eq!(request.about().as_str(), "about:0");
        // 3 abouts * (1 wake + 5 expansions) + 1 terminal = 19.
        assert_eq!(request.initial_budget().calls_remaining(), 19);
        // Starts blind: the operator must wake the entry about itself.
        assert!(
            !request
                .initial_visible_state()
                .knows_ref(&MemoryRef::parse("node:0").unwrap())
        );
    }

    #[test]
    fn budget_grows_with_more_abouts() {
        let session_id = OperatorSessionId::parse("xab:0002").unwrap();
        let request = CrossAboutEpisodeCompiler::compile(session_id, &episode(1, 5)).unwrap();
        // 1 about * (1 + 5) + 1 = 7.
        assert_eq!(request.initial_budget().calls_remaining(), 7);
    }
}
