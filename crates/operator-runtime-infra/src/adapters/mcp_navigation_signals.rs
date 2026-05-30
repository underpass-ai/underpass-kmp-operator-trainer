//! Parse the intrinsic context-coverage signals out of a KMP/MCP tool-result
//! JSON (`coverage`, `proof`, `page`, `quality`, `answer`, `links`) into a
//! [`NavigationSignals`]. Shared by the HTTP and stdio MCP executors. Missing
//! fields default to "no shortfall" so older/partial backends degrade to
//! neutral signals rather than failing.

use std::collections::BTreeSet;

use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::navigation_signals::NavigationSignals;
use serde_json::Value;

pub(crate) fn navigation_signals_from_structured(
    structured: &Value,
    observed_refs: &[MemoryRef],
) -> NavigationSignals {
    let coverage = structured.get("coverage");
    let included = array_len(coverage, "included");
    let missing = array_len(coverage, "missing");

    let page = structured.get("page");
    let page_has_more = bool_at(page, "has_more");
    let page_returned = u32_at(page, "returned");
    let page_total = u32_at(page, "total");

    let proof = structured.get("proof");
    let frontier_size = u32_at(proof, "frontier_size");
    let conflicts = count_u32(array_len(proof, "conflicts"));
    let confidence_is_unknown = proof
        .and_then(|proof| proof.get("confidence"))
        .and_then(Value::as_str)
        == Some("unknown");

    let answer_is_null = matches!(structured.get("answer"), Some(Value::Null));

    let observed: BTreeSet<&str> = observed_refs.iter().map(MemoryRef::as_str).collect();
    let link_targets = inspect_link_targets(structured);
    let total_links = count_u32(link_targets.len());
    let unvisited_links = count_u32(
        link_targets
            .iter()
            .filter(|target| !observed.contains(target.as_str()))
            .count(),
    );

    NavigationSignals::new(
        count_u32(included + missing),
        count_u32(missing),
        page_has_more,
        page_returned,
        page_total,
        frontier_size,
        conflicts,
        confidence_is_unknown,
        answer_is_null,
        unvisited_links,
        total_links,
    )
}

fn inspect_link_targets(structured: &Value) -> Vec<String> {
    let mut targets = Vec::new();
    if let Some(links) = structured.get("links") {
        for direction in ["incoming", "outgoing"] {
            if let Some(relations) = links.get(direction).and_then(Value::as_array) {
                for relation in relations {
                    for endpoint in ["from", "to"] {
                        if let Some(value) = relation
                            .get(endpoint)
                            .and_then(Value::as_str)
                            .filter(|value| !value.is_empty())
                        {
                            targets.push(value.to_string());
                        }
                    }
                }
            }
        }
    }
    targets
}

fn array_len(parent: Option<&Value>, key: &str) -> usize {
    parent
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn bool_at(parent: Option<&Value>, key: &str) -> bool {
    parent
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn u32_at(parent: Option<&Value>, key: &str) -> u32 {
    parent
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}

fn count_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_coverage_proof_page_and_links() {
        let structured = json!({
            "coverage": { "included": ["a"], "missing": ["b", "c"] },
            "page": { "returned": 3, "total": 10, "has_more": true },
            "proof": { "frontier_size": 2, "conflicts": ["x"], "confidence": "unknown" },
            "links": {
                "incoming": [{ "from": "node:1", "to": "node:self" }],
                "outgoing": [{ "from": "node:self", "to": "node:2" }]
            }
        });
        let observed = vec![MemoryRef::parse("node:self").unwrap()];
        let signals = navigation_signals_from_structured(&structured, &observed);

        assert_eq!(signals.coverage_requested(), 3);
        assert_eq!(signals.coverage_missing(), 2);
        assert!(signals.page_has_more());
        assert_eq!(signals.page_total(), 10);
        assert_eq!(signals.frontier_size(), 2);
        assert_eq!(signals.conflicts(), 1);
        assert!(signals.confidence_is_unknown());
        assert_eq!(signals.total_links(), 4);
        // node:1, node:2 are unvisited; the two node:self endpoints are observed.
        assert_eq!(signals.unvisited_links(), 2);
    }

    #[test]
    fn missing_fields_degrade_to_neutral() {
        let signals = navigation_signals_from_structured(&json!({}), &[]);
        assert_eq!(signals.coverage_requested(), 0);
        assert!(!signals.page_has_more());
        assert!(!signals.confidence_is_unknown());
        assert!(!signals.answer_is_null());
    }

    #[test]
    fn null_answer_is_detected() {
        let signals = navigation_signals_from_structured(&json!({ "answer": null }), &[]);
        assert!(signals.answer_is_null());
    }
}
