use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact, field_result_exact_debug,
};
use crate::tool_arguments::rewind_arguments::RewindArguments;

impl ActionCorrectness for RewindArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_exact(
                "cursor.key",
                self.cursor().key().as_str().to_string(),
                ground_truth.cursor().key().as_str().to_string(),
            ),
            field_result_exact(
                "cursor.anchor",
                self.cursor().anchor().as_str().to_string(),
                ground_truth.cursor().anchor().as_str().to_string(),
            ),
            field_result_exact_debug("window", &self.window(), &ground_truth.window()),
        ])
    }
}
