use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;

pub trait ActionCorrectness {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome;
}
