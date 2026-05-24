use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::tool_arguments::tool_arguments::ToolArguments;

impl ActionCorrectness for ToolArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        match (self, ground_truth) {
            (Self::Ingest(actual), Self::Ingest(expected)) => actual.evaluate_correctness(expected),
            (Self::Wake(actual), Self::Wake(expected)) => actual.evaluate_correctness(expected),
            (Self::Ask(actual), Self::Ask(expected)) => actual.evaluate_correctness(expected),
            (Self::Near(actual), Self::Near(expected)) => actual.evaluate_correctness(expected),
            (Self::Goto(actual), Self::Goto(expected)) => actual.evaluate_correctness(expected),
            (Self::Rewind(actual), Self::Rewind(expected)) => actual.evaluate_correctness(expected),
            (Self::Forward(actual), Self::Forward(expected)) => {
                actual.evaluate_correctness(expected)
            }
            (Self::Trace(actual), Self::Trace(expected)) => actual.evaluate_correctness(expected),
            (Self::Inspect(actual), Self::Inspect(expected)) => {
                actual.evaluate_correctness(expected)
            }
            (Self::WriteMemory(actual), Self::WriteMemory(expected)) => {
                actual.evaluate_correctness(expected)
            }
            _ => ActionCorrectnessOutcome::tool_mismatch(self.tool(), ground_truth.tool()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::cursor::Cursor;
    use crate::cursor::ref_cursor::RefCursor;
    use crate::cursor::temporal_anchor::TemporalAnchor;
    use crate::cursor::temporal_cursor::TemporalCursor;
    use crate::cursor::temporal_cursor_key::TemporalCursorKey;
    use crate::tool_arguments::ask_arguments::AskArguments;
    use crate::tool_arguments::forward_arguments::ForwardArguments;
    use crate::tool_arguments::goto_arguments::GotoArguments;
    use crate::tool_arguments::inspect_arguments::InspectArguments;
    use crate::tool_arguments::near_arguments::NearArguments;
    use crate::tool_arguments::rewind_arguments::RewindArguments;
    use crate::tool_arguments::trace_arguments::TraceArguments;
    use crate::tool_arguments::wake_arguments::WakeArguments;
    use crate::tool_arguments::write_memory_arguments::WriteMemoryArguments;
    use crate::value_objects::dimension_ref::DimensionRef;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::positive_count::PositiveCount;

    fn memory_ref(value: &str) -> MemoryRef {
        MemoryRef::parse(value).unwrap()
    }

    fn dimension_ref(value: &str) -> DimensionRef {
        DimensionRef::parse(value).unwrap()
    }

    fn count(value: usize) -> PositiveCount {
        PositiveCount::parse(value, "test").unwrap()
    }

    fn temporal_cursor(key: TemporalCursorKey, anchor: &str) -> TemporalCursor {
        TemporalCursor::new(key, TemporalAnchor::parse(anchor).unwrap())
    }

    #[test]
    fn variant_mismatch_reports_tool_mismatch() {
        let actual = ToolArguments::Inspect(InspectArguments::new(memory_ref("node:1")));
        let expected = ToolArguments::Ask(AskArguments::new("why").unwrap());

        let outcome = actual.evaluate_correctness(&expected);

        assert!(!outcome.is_correct());
        assert_eq!(
            outcome
                .failed_fields()
                .next()
                .unwrap()
                .field_path()
                .as_str(),
            "tool"
        );
    }

    #[test]
    fn simple_exact_tools_detect_field_mismatch() {
        let actual = ToolArguments::Wake(WakeArguments::new(
            crate::ids::about_id::AboutId::parse("about:actual").unwrap(),
        ));
        let expected = ToolArguments::Wake(WakeArguments::new(
            crate::ids::about_id::AboutId::parse("about:expected").unwrap(),
        ));

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn ask_query_is_permissive() {
        let actual = ToolArguments::Ask(AskArguments::new("actual query").unwrap());
        let expected = ToolArguments::Ask(AskArguments::new("expected query").unwrap());

        assert!(actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn near_dimensions_are_exact() {
        let actual = ToolArguments::Near(
            NearArguments::new(memory_ref("node:1"), vec![dimension_ref("semantic")], None)
                .unwrap(),
        );
        let expected = ToolArguments::Near(
            NearArguments::new(memory_ref("node:1"), vec![dimension_ref("temporal")], None)
                .unwrap(),
        );

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn goto_cursor_kind_and_payload_are_exact() {
        let actual = ToolArguments::Goto(GotoArguments::new(Cursor::Ref(RefCursor::new(
            memory_ref("node:1"),
        ))));
        let expected = ToolArguments::Goto(GotoArguments::new(Cursor::Temporal(temporal_cursor(
            TemporalCursorKey::Created,
            "2026-05-23T00:00:00Z",
        ))));

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn temporal_navigation_window_is_exact() {
        let cursor = temporal_cursor(TemporalCursorKey::Created, "2026-05-23T00:00:00Z");
        let actual = ToolArguments::Rewind(RewindArguments::new(cursor.clone(), count(1)));
        let expected = ToolArguments::Rewind(RewindArguments::new(cursor, count(2)));

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn forward_cursor_is_exact() {
        let actual = ToolArguments::Forward(ForwardArguments::new(
            temporal_cursor(TemporalCursorKey::Created, "2026-05-23T00:00:00Z"),
            count(1),
        ));
        let expected = ToolArguments::Forward(ForwardArguments::new(
            temporal_cursor(TemporalCursorKey::Updated, "2026-05-23T00:00:00Z"),
            count(1),
        ));

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn trace_refs_are_exact() {
        let actual = ToolArguments::Trace(TraceArguments::new(
            memory_ref("node:from"),
            Some(memory_ref("node:actual")),
            count(1),
        ));
        let expected = ToolArguments::Trace(TraceArguments::new(
            memory_ref("node:from"),
            Some(memory_ref("node:expected")),
            count(1),
        ));

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn write_memory_text_is_permissive_but_related_refs_are_exact() {
        let actual = ToolArguments::WriteMemory(
            WriteMemoryArguments::new(
                "different summary",
                "different body",
                vec![memory_ref("node:1")],
            )
            .unwrap(),
        );
        let expected = ToolArguments::WriteMemory(
            WriteMemoryArguments::new("summary", "body", vec![memory_ref("node:1")]).unwrap(),
        );
        assert!(actual.evaluate_correctness(&expected).is_correct());

        let wrong_related = ToolArguments::WriteMemory(
            WriteMemoryArguments::new("summary", "body", vec![memory_ref("node:2")]).unwrap(),
        );
        assert!(!wrong_related.evaluate_correctness(&expected).is_correct());
    }
}
