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
