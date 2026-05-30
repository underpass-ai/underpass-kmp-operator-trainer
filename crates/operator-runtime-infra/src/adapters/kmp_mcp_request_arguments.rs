use chrono::DateTime;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpRequestArgumentsError {
    #[error("kernel_trace requires a target `to` ref for live KMP execution")]
    TraceTargetRequired,

    #[error("trace cursors cannot be used as kernel_goto temporal positions")]
    TraceCursorUnsupportedForGoto,

    #[error("invalid temporal sequence anchor `{anchor}`; expected seq:<positive-u32>")]
    InvalidTemporalSequence { anchor: String },
}

pub fn wake(args: &WakeArguments) -> Value {
    json!({ "about": args.about().as_str() })
}

pub fn ask(args: &AskArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "question": args.query().as_str(),
    })
}

pub fn near(args: &NearArguments, about: &AboutId) -> Value {
    let mut body = json!({
        "about": about.as_str(),
        "around": { "ref": args.anchor().as_str() },
        "dimensions": dimensions(args.dimensions()),
    });
    if let Some(limit) = args.limit() {
        body["limit"] = json!({ "entries": limit.as_usize() });
    }
    // The kernel's Near is driven by the temporal window (before/after entries),
    // which is the lever for window expansion. Send it when set.
    if args.before_entries() != 0 || args.after_entries() != 0 {
        body["window"] = json!({
            "before_entries": args.before_entries(),
            "after_entries": args.after_entries(),
        });
    }
    body
}

pub fn goto(args: &GotoArguments, about: &AboutId) -> Result<Value, McpRequestArgumentsError> {
    let mut body = json!({
        "about": about.as_str(),
        "at": cursor_to_temporal_position(args.cursor())?,
    });
    if let Cursor::Around(cursor) = args.cursor() {
        body["dimensions"] = dimensions(cursor.dimensions());
    }
    Ok(body)
}

pub fn rewind(args: &RewindArguments, about: &AboutId) -> Result<Value, McpRequestArgumentsError> {
    Ok(json!({
        "about": about.as_str(),
        "from": temporal_anchor(args.cursor().anchor().as_str())?,
        "window": { "before_entries": args.window().as_usize() },
    }))
}

pub fn forward(
    args: &ForwardArguments,
    about: &AboutId,
) -> Result<Value, McpRequestArgumentsError> {
    Ok(json!({
        "about": about.as_str(),
        "from": temporal_anchor(args.cursor().anchor().as_str())?,
        "window": { "after_entries": args.window().as_usize() },
    }))
}

pub fn trace(args: &TraceArguments) -> Result<Value, McpRequestArgumentsError> {
    let to = args
        .to()
        .ok_or(McpRequestArgumentsError::TraceTargetRequired)?;
    Ok(json!({
        "from": args.from().as_str(),
        "to": to.as_str(),
        "page": { "entries": args.page().as_usize() },
    }))
}

pub fn inspect(args: &InspectArguments) -> Value {
    json!({ "ref": args.target().as_str() })
}

fn cursor_to_temporal_position(cursor: &Cursor) -> Result<Value, McpRequestArgumentsError> {
    match cursor {
        Cursor::Ref(cursor) => Ok(json!({ "ref": cursor.target().as_str() })),
        Cursor::Around(cursor) => Ok(json!({ "ref": cursor.anchor().as_str() })),
        Cursor::Temporal(cursor) => temporal_anchor(cursor.anchor().as_str()),
        Cursor::Trace(_) => Err(McpRequestArgumentsError::TraceCursorUnsupportedForGoto),
    }
}

fn temporal_anchor(anchor: &str) -> Result<Value, McpRequestArgumentsError> {
    if let Some(raw) = anchor.strip_prefix("seq:") {
        let sequence = raw
            .parse::<u32>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| McpRequestArgumentsError::InvalidTemporalSequence {
                anchor: anchor.to_string(),
            })?;
        return Ok(json!({ "sequence": sequence }));
    }
    if DateTime::parse_from_rfc3339(anchor).is_ok() {
        return Ok(json!({ "time": anchor }));
    }
    Ok(json!({ "ref": anchor }))
}

fn dimensions(dims: &[DimensionRef]) -> Value {
    json!({
        "mode": "only",
        "include": dims.iter().map(DimensionRef::as_str).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
    use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
    use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
    use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
    use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    use super::*;

    #[test]
    fn trace_uses_required_to_and_page_object() {
        let args = TraceArguments::new(
            MemoryRef::parse("node:from").unwrap(),
            Some(MemoryRef::parse("node:to").unwrap()),
            PositiveCount::parse(7, "page").unwrap(),
        );

        let body = trace(&args).unwrap();

        assert_eq!(body["from"], "node:from");
        assert_eq!(body["to"], "node:to");
        assert_eq!(body["page"]["entries"], 7);
    }

    #[test]
    fn trace_without_to_is_rejected_before_transport() {
        let args = TraceArguments::new(
            MemoryRef::parse("node:from").unwrap(),
            None,
            PositiveCount::parse(1, "page").unwrap(),
        );

        assert_eq!(
            trace(&args),
            Err(McpRequestArgumentsError::TraceTargetRequired)
        );
    }

    #[test]
    fn temporal_anchor_accepts_rfc3339_offsets_without_false_positive_refs() {
        assert_eq!(
            temporal_anchor("2026-05-26T12:00:00+00:00").unwrap(),
            json!({ "time": "2026-05-26T12:00:00+00:00" })
        );
        assert_eq!(
            temporal_anchor("evidence:abcT123Z").unwrap(),
            json!({ "ref": "evidence:abcT123Z" })
        );
    }

    #[test]
    fn temporal_anchor_rejects_zero_sequence() {
        assert_eq!(
            temporal_anchor("seq:0"),
            Err(McpRequestArgumentsError::InvalidTemporalSequence {
                anchor: "seq:0".to_string()
            })
        );
    }

    #[test]
    fn forward_uses_temporal_sequence_shape() {
        let cursor = TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse("seq:3").unwrap(),
        );
        let args = ForwardArguments::new(cursor, PositiveCount::parse(2, "window").unwrap());
        let about = AboutId::parse("about:test").unwrap();

        let body = forward(&args, &about).unwrap();

        assert_eq!(body["about"], "about:test");
        assert_eq!(body["from"]["sequence"], 3);
        assert_eq!(body["window"]["after_entries"], 2);
    }
}
