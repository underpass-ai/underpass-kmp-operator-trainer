use crate::action::stop_action::StopAction;
use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact, field_result_exact_debug, field_result_permissive_optional,
};

impl ActionCorrectness for StopAction {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(vec![
            field_result_exact("kind", "stop".to_string(), "stop".to_string()),
            field_result_exact(
                "reason",
                self.reason().as_str().to_string(),
                ground_truth.reason().as_str().to_string(),
            ),
            field_result_permissive_optional("answer", self.answer(), ground_truth.answer()),
            field_result_exact_debug("evidence", &self.evidence(), &ground_truth.evidence()),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::stop_reason::StopReason;
    use crate::value_objects::memory_ref::MemoryRef;

    #[test]
    fn detects_exact_reason_mismatch() {
        let actual = StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap();
        let expected = StopAction::new(StopReason::NoCandidate, None, vec![]).unwrap();
        let outcome = actual.evaluate_correctness(&expected);

        assert!(!outcome.is_correct());
        assert_eq!(
            outcome
                .failed_fields()
                .next()
                .unwrap()
                .field_path()
                .as_str(),
            "reason"
        );
    }

    #[test]
    fn accepts_permissive_answer_text() {
        let actual = StopAction::new(
            StopReason::AnswerReady,
            Some("different".to_string()),
            vec![],
        )
        .unwrap();
        let expected = StopAction::new(
            StopReason::AnswerReady,
            Some("expected".to_string()),
            vec![],
        )
        .unwrap();

        assert!(actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn evidence_remains_exact() {
        let actual = StopAction::new(
            StopReason::AnswerReady,
            None,
            vec![MemoryRef::parse("node:actual").unwrap()],
        )
        .unwrap();
        let expected = StopAction::new(
            StopReason::AnswerReady,
            None,
            vec![MemoryRef::parse("node:expected").unwrap()],
        )
        .unwrap();

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }
}
