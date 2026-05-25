use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact, field_result_exact_debug,
};
use crate::tool_arguments::near_arguments::NearArguments;

impl ActionCorrectness for NearArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_exact(
                "anchor",
                self.anchor().as_str().to_string(),
                ground_truth.anchor().as_str().to_string(),
            ),
            field_result_exact_debug(
                "dimensions[*]",
                &self.dimensions(),
                &ground_truth.dimensions(),
            ),
            field_result_exact_debug("limit", &self.limit(), &ground_truth.limit()),
        ])
    }
}
