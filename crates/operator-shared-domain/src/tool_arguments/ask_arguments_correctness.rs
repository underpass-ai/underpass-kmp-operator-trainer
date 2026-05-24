use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::field_result_permissive_required;
use crate::tool_arguments::ask_arguments::AskArguments;

impl ActionCorrectness for AskArguments {
    fn evaluate_correctness(&self, _ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![field_result_permissive_required(
            "query",
            self.query(),
        )])
    }
}
