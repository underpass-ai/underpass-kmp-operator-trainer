use crate::action::tool_call_action::ToolCallAction;
use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;

impl ActionCorrectness for ToolCallAction {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        if self.tool() != ground_truth.tool() {
            return ActionCorrectnessOutcome::tool_mismatch(self.tool(), ground_truth.tool());
        }
        self.arguments()
            .evaluate_correctness(ground_truth.arguments())
    }
}
