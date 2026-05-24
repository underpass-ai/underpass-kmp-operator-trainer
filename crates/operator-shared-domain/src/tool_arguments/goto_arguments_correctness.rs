use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::field_result_helpers::field_result_exact;
use crate::cursor::cursor::Cursor;
use crate::tool_arguments::goto_arguments::GotoArguments;

impl ActionCorrectness for GotoArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        ActionCorrectnessOutcome::new(cursor_results(self.cursor(), ground_truth.cursor()))
    }
}

fn cursor_results(
    actual: &Cursor,
    expected: &Cursor,
) -> Vec<crate::contract::correctness::field_result::FieldResult> {
    let mut results = vec![field_result_exact(
        "cursor.kind",
        actual.kind().as_str().to_string(),
        expected.kind().as_str().to_string(),
    )];
    match (actual, expected) {
        (Cursor::Ref(actual), Cursor::Ref(expected)) => results.push(field_result_exact(
            "cursor.target",
            actual.target().as_str().to_string(),
            expected.target().as_str().to_string(),
        )),
        (Cursor::Around(actual), Cursor::Around(expected)) => {
            results.push(field_result_exact(
                "cursor.anchor",
                actual.anchor().as_str().to_string(),
                expected.anchor().as_str().to_string(),
            ));
            results.push(
                crate::contract::correctness::field_result_helpers::field_result_exact_debug(
                    "cursor.dimensions[*]",
                    &actual.dimensions(),
                    &expected.dimensions(),
                ),
            );
        }
        (Cursor::Temporal(actual), Cursor::Temporal(expected)) => {
            results.push(field_result_exact(
                "cursor.key",
                actual.key().as_str().to_string(),
                expected.key().as_str().to_string(),
            ));
            results.push(field_result_exact(
                "cursor.anchor",
                actual.anchor().as_str().to_string(),
                expected.anchor().as_str().to_string(),
            ));
        }
        (Cursor::Trace(actual), Cursor::Trace(expected)) => {
            results.push(field_result_exact(
                "cursor.from",
                actual.from().as_str().to_string(),
                expected.from().as_str().to_string(),
            ));
            results.push(field_result_exact(
                "cursor.to",
                actual.to().as_str().to_string(),
                expected.to().as_str().to_string(),
            ));
        }
        _ => {}
    }
    results
}
