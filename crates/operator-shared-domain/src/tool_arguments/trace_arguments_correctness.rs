use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact, field_result_exact_debug,
};
use crate::tool_arguments::trace_arguments::TraceArguments;

impl ActionCorrectness for TraceArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_exact(
                "from",
                self.from().as_str().to_string(),
                ground_truth.from().as_str().to_string(),
            ),
            field_result_exact_debug("to", &self.to(), &ground_truth.to()),
            field_result_exact_debug("page", &self.page(), &ground_truth.page()),
        ])
    }
}
