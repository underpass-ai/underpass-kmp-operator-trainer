use crate::action::escalate_action::EscalateAction;
use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::field_result_exact;

impl ActionCorrectness for EscalateAction {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_exact("kind", "escalate".to_string(), "escalate".to_string()),
            field_result_exact(
                "reason",
                self.reason().as_str().to_string(),
                ground_truth.reason().as_str().to_string(),
            ),
            field_result_exact(
                "target_model",
                self.target_model().as_str().to_string(),
                ground_truth.target_model().as_str().to_string(),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::escalate_reason::EscalateReason;
    use crate::value_objects::model_id::ModelId;

    #[test]
    fn target_model_is_exact() {
        let actual = EscalateAction::new(
            EscalateReason::LowConfidence,
            ModelId::parse("gpt-4o-mini").unwrap(),
        );
        let expected = EscalateAction::new(
            EscalateReason::LowConfidence,
            ModelId::parse("gpt-4o").unwrap(),
        );

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }
}
