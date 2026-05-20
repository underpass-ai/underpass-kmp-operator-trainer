//! Join `StepKeyedPrediction`s back to their ground-truth
//! `TrainingTrajectory` set via `step_id`, producing the
//! `EvaluationPair` values the policy evaluator scores.
//!
//! Predictions for steps not in the ground-truth set are dropped
//! silently — matches the kernel evaluator's contract when a holdout
//! row is missing. Ground-truth duplicates surface as a debug-time
//! panic: a duplicated `step_id` is a caller invariant violation,
//! never expected in production data; in release builds the second
//! trajectory wins, matching the previous in-memory `HashMap`
//! behaviour, so we never panic on user data.

use std::collections::HashMap;

use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::prediction::evaluation_pair::EvaluationPair;
use crate::prediction::predicted_action::PredictedAction;
use crate::prediction::step_keyed_prediction::StepKeyedPrediction;

#[must_use]
pub fn join_step_predictions(
    ground_truth: &[TrainingTrajectory],
    predictions: &[StepKeyedPrediction],
) -> Vec<EvaluationPair> {
    let mut truth_by_step: HashMap<&StepId, &TrainingTrajectory> =
        HashMap::with_capacity(ground_truth.len());
    for trajectory in ground_truth {
        let previous = truth_by_step.insert(trajectory.step_id(), trajectory);
        debug_assert!(
            previous.is_none(),
            "ground truth has duplicate step_id `{}` — caller invariant violated",
            trajectory.step_id().as_str()
        );
    }
    let mut pairs = Vec::new();
    for prediction in predictions {
        let Some(trajectory) = truth_by_step.get(prediction.step_id()) else {
            continue;
        };
        let predicted = PredictedAction::new(trajectory.id().clone(), prediction.action().clone());
        // EvaluationPair::new is constructed with the matched
        // trajectory's id, so its id check cannot fail here.
        let pair = EvaluationPair::new((*trajectory).clone(), predicted)
            .expect("EvaluationPair: ids match by construction");
        pairs.push(pair);
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    fn inspect_trajectory(traj_id: &str, step_id: &str, memory: &str) -> TrainingTrajectory {
        let target = MemoryRef::parse(memory).unwrap();
        let visible =
            VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(traj_id).unwrap(),
            StepId::parse(step_id).unwrap(),
            AboutId::parse("a:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
        .unwrap()
    }

    fn inspect_prediction(step_id: &str, memory: &str) -> StepKeyedPrediction {
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse(memory).unwrap()),
        )));
        StepKeyedPrediction::new(StepId::parse(step_id).unwrap(), action)
    }

    #[test]
    fn empty_inputs_produce_no_pairs() {
        let pairs = join_step_predictions(&[], &[]);
        assert!(pairs.is_empty());
    }

    #[test]
    fn empty_predictions_against_populated_truth_produce_no_pairs() {
        let truth = vec![inspect_trajectory("t:1", "s:1", "node:1")];
        let pairs = join_step_predictions(&truth, &[]);
        assert!(pairs.is_empty());
    }

    #[test]
    fn predictions_with_no_matching_truth_are_dropped_silently() {
        // Predictions exist but none reference a step_id in the truth
        // set. Documented contract: drop silently.
        let truth = vec![inspect_trajectory("t:1", "s:1", "node:1")];
        let predictions = vec![inspect_prediction("s:unknown", "node:1")];
        let pairs = join_step_predictions(&truth, &predictions);
        assert!(pairs.is_empty());
    }

    #[test]
    fn perfect_1_to_1_match_returns_one_pair_per_prediction() {
        let truth = vec![
            inspect_trajectory("t:1", "s:1", "node:1"),
            inspect_trajectory("t:2", "s:2", "node:2"),
        ];
        let predictions = vec![
            inspect_prediction("s:1", "node:1"),
            inspect_prediction("s:2", "node:2"),
        ];
        let pairs = join_step_predictions(&truth, &predictions);
        assert_eq!(pairs.len(), 2);
        // The pair carries the trajectory's id, not the prediction's
        // step_id, so callers downstream can group by trajectory.
        let trajectory_ids: Vec<String> = pairs
            .iter()
            .map(|pair| pair.ground_truth().id().as_str().to_string())
            .collect();
        assert!(trajectory_ids.contains(&"t:1".to_string()));
        assert!(trajectory_ids.contains(&"t:2".to_string()));
    }

    #[test]
    fn truth_rows_without_matching_predictions_are_skipped() {
        // The function is driven by predictions: a truth row with no
        // prediction does not yield a pair. This is the kernel
        // evaluator's contract — uncovered truth rows simply don't
        // count toward the score.
        let truth = vec![
            inspect_trajectory("t:1", "s:1", "node:1"),
            inspect_trajectory("t:2", "s:2", "node:2"),
        ];
        let predictions = vec![inspect_prediction("s:1", "node:1")];
        let pairs = join_step_predictions(&truth, &predictions);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].ground_truth().id().as_str(), "t:1");
    }

    #[test]
    #[should_panic(expected = "duplicate step_id")]
    fn duplicate_step_id_in_truth_panics_in_debug_builds() {
        // Caller-invariant violation: same step_id on two truth rows.
        // The debug_assert fires in test (debug) builds; release
        // builds let the second row win silently.
        let truth = vec![
            inspect_trajectory("t:1", "s:duplicate", "node:1"),
            inspect_trajectory("t:2", "s:duplicate", "node:2"),
        ];
        let _ = join_step_predictions(&truth, &[]);
    }
}
