use crate::action::operator_action::OperatorAction;
use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;

impl ActionCorrectness for OperatorAction {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        match (self, ground_truth) {
            (Self::ToolCall(actual), Self::ToolCall(expected)) => {
                actual.evaluate_correctness(expected)
            }
            (Self::Stop(actual), Self::Stop(expected)) => actual.evaluate_correctness(expected),
            (Self::Escalate(actual), Self::Escalate(expected)) => {
                actual.evaluate_correctness(expected)
            }
            _ => ActionCorrectnessOutcome::kind_mismatch(self.kind(), ground_truth.kind()),
        }
    }
}
