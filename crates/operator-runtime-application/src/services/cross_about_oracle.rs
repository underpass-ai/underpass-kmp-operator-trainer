//! Deterministic, gold-based verification that a cross-about session retrieved
//! every about's operand set.
//!
//! Across abouts the kernel's last-move in-band signals only reflect the final
//! about, so — unlike the single-about [`window_expansion_oracle`] — coverage
//! cannot be read from signals. The cross-about oracle instead checks the gold
//! directly: every target about's expected entries must be present in the
//! session's accumulated visible state (refs are namespaced per about, so the
//! flat accumulation is an exact cross-about union). It reports which abouts
//! fell short. Like every oracle it is a label/verify step the generator uses
//! to accept or drop a transcript; it is never an SFT target.
//!
//! [`window_expansion_oracle`]: crate::services::window_expansion_oracle

use operator_runtime_domain::session::cross_about_coverage_outcome::CrossAboutCoverageOutcome;
use operator_runtime_domain::session::session_transcript::SessionTranscript;
use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;

#[derive(Debug)]
pub struct CrossAboutOracle;

impl CrossAboutOracle {
    /// Classify whether every target about's gold entries were retrieved. An
    /// about is uncovered when any of its expected refs is missing from the
    /// final visible state.
    pub fn verify(
        episode: &CrossAboutEpisode,
        transcript: &SessionTranscript,
    ) -> CrossAboutCoverageOutcome {
        let final_state = transcript.final_visible_state();
        let mut uncovered = Vec::new();
        for target in episode.targets() {
            let covered = target
                .expected_refs()
                .iter()
                .all(|memory_ref| final_state.knows_ref(memory_ref));
            if !covered {
                uncovered.push(target.about().clone());
            }
        }
        if uncovered.is_empty() {
            CrossAboutCoverageOutcome::Complete
        } else {
            CrossAboutCoverageOutcome::Incomplete {
                uncovered_abouts: uncovered,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use operator_runtime_domain::budget::session_budget::SessionBudget;
    use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_domain::episode::cross_about_target::CrossAboutTarget;
    use operator_synthetic_domain::episode::window_expansion_spec::WindowExpansionSpec;

    fn episode() -> CrossAboutEpisode {
        CrossAboutEpisode::new(
            vec![
                CrossAboutTarget::new(
                    AboutId::parse("about:eu").unwrap(),
                    vec![MemoryRef::parse("node:eu1").unwrap()],
                ),
                CrossAboutTarget::new(
                    AboutId::parse("about:us").unwrap(),
                    vec![MemoryRef::parse("node:us1").unwrap()],
                ),
            ],
            TrajectoryGoal::parse("Count across EU and US.").unwrap(),
            WindowExpansionSpec::new(
                PositiveCount::parse(4, "window").unwrap(),
                PositiveCount::parse(5, "iterations").unwrap(),
            ),
            8192,
        )
    }

    fn transcript_knowing(refs: &[&str]) -> SessionTranscript {
        let known: Vec<MemoryRef> = refs.iter().map(|r| MemoryRef::parse(*r).unwrap()).collect();
        let state = VisibleState::assemble(known, [], None, BudgetSnapshot::bounded(1, 4096));
        SessionTranscript::completed(
            OperatorSessionId::parse("xab:oracle").unwrap(),
            Vec::new(),
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap()),
            state,
            SessionBudget::new(1, 4096),
            Duration::from_millis(1),
            AboutId::parse("about:us").unwrap(),
        )
    }

    #[test]
    fn complete_when_every_about_gold_is_retrieved() {
        let outcome = CrossAboutOracle::verify(&episode(), &transcript_knowing(&["node:eu1", "node:us1"]));
        assert!(outcome.is_covered());
    }

    #[test]
    fn incomplete_lists_the_about_left_short() {
        // US gold missing.
        let outcome = CrossAboutOracle::verify(&episode(), &transcript_knowing(&["node:eu1"]));
        assert!(!outcome.is_covered());
        assert_eq!(outcome.uncovered_abouts().len(), 1);
        assert_eq!(outcome.uncovered_abouts()[0].as_str(), "about:us");
    }
}
