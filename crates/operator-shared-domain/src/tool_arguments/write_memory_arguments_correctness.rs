use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact_debug, field_result_permissive_required,
};
use crate::tool_arguments::write_memory_arguments::WriteMemoryArguments;

impl ActionCorrectness for WriteMemoryArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_permissive_required("summary", self.summary()),
            field_result_permissive_required("body", self.body()),
            field_result_exact_debug("related[*]", &self.related(), &ground_truth.related()),
        ])
    }
}
