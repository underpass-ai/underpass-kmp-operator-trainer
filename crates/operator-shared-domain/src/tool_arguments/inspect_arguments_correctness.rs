use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::field_result_exact;
use crate::tool_arguments::inspect_arguments::InspectArguments;

impl ActionCorrectness for InspectArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![field_result_exact(
            "target",
            self.target().as_str().to_string(),
            ground_truth.target().as_str().to_string(),
        )])
    }
}
