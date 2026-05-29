//! Per-request reference anonymization for serving the anonymized operator.
//!
//! The release model is trained on opaque, domain-free refs (`about_0001`,
//! `ref_0001`, ...) so that it learns to *use KMP* rather than memorize domain
//! topics (see `docs/training/DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`).
//! Live requests, however, carry real domain refs (`about:incident:...`). This
//! value object builds a per-request bijection between the real refs visible in a
//! subject and stable opaque ids, so the serving path can anonymize the model-
//! facing request and de-anonymize the predicted action back to real refs (the
//! V6 "raw-ref de-anonymization" design).
//!
//! It works on the serialized DTO `serde_json::Value` (an infra concern; the
//! domain never sees serde), substring-rewriting refs everywhere they occur —
//! structured fields AND free text such as the goal. KMP *dimension* kinds
//! (`agent:`/`task:`/`topic:`/`session:`/`attempt:`) and server-issued temporal
//! anchors (`seq:5`, timestamps) are NOT refs and are left untouched.

use serde_json::Value;

/// Domain-topic ref prefixes that must be anonymized out of model-facing state.
/// Mirrors `prepare_operator_sft_dataset.py` `DOMAIN_TOPIC_REF_PREFIXES` and
/// `deanonymize_operator_predictions.py`. Deliberately excludes dimension kinds.
const DOMAIN_REF_PREFIXES: &[&str] = &[
    "about:",
    "incident:",
    "migration:",
    "bug:",
    "product:",
    "docs:",
    "evidence:",
    "question:run:",
    "turn:run:",
    "memoryarena:run:",
    "longmemeval:",
    "memoryagentbench:",
];

fn looks_like_domain_ref(value: &str) -> bool {
    DOMAIN_REF_PREFIXES
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

#[derive(Debug, Clone)]
pub struct RefAnonymization {
    /// (real -> opaque), sorted by real length descending so a scope that is a
    /// prefix of a longer node ref never corrupts it during substring rewrite.
    forward: Vec<(String, String)>,
    /// (opaque -> real), sorted by opaque length descending.
    reverse: Vec<(String, String)>,
}

impl RefAnonymization {
    /// Build the map from the subject's real refs. `about` takes the `about_NNNN`
    /// id space; every other domain ref (the visible `known_refs` plus any
    /// domain-ref string value discovered anywhere in `subject_value`, e.g. cursor
    /// targets or prepared-action refs) takes the `ref_NNNN` space. Assignment
    /// order is deterministic (`known_refs` sorted, then discovery order).
    pub fn build(about: &str, known_refs: &[String], subject_value: &Value) -> Self {
        let mut pairs: Vec<(String, String)> = Vec::new();
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        if !about.is_empty() && seen.insert(about.to_string()) {
            pairs.push((about.to_string(), "about_0001".to_string()));
        }

        let mut ref_idx = 0usize;
        let assign_ref = |real: &str,
                          pairs: &mut Vec<(String, String)>,
                          seen: &mut std::collections::BTreeSet<String>,
                          ref_idx: &mut usize| {
            if seen.insert(real.to_string()) {
                *ref_idx += 1;
                pairs.push((real.to_string(), format!("ref_{ref_idx:04}")));
            }
        };

        let mut sorted_known: Vec<&String> = known_refs.iter().collect();
        sorted_known.sort();
        for real in sorted_known {
            assign_ref(real, &mut pairs, &mut seen, &mut ref_idx);
        }

        // Completeness: catch any standalone domain-ref string value (e.g. a cursor
        // target or prepared-action ref) not already in known_refs.
        collect_domain_ref_values(subject_value, &mut |s| {
            if !seen.contains(s) {
                assign_ref(s, &mut pairs, &mut seen, &mut ref_idx);
            }
        });

        let mut forward = pairs.clone();
        forward.sort_by_key(|pair| std::cmp::Reverse(pair.0.len()));
        let mut reverse: Vec<(String, String)> = pairs
            .into_iter()
            .map(|(real, opaque)| (opaque, real))
            .collect();
        reverse.sort_by_key(|pair| std::cmp::Reverse(pair.0.len()));

        Self { forward, reverse }
    }

    /// Rewrite real refs -> opaque ids across the whole value.
    pub fn anonymize(&self, value: &Value) -> Value {
        rewrite_value(value, &self.forward)
    }

    /// Rewrite opaque ids -> real refs across the whole value. Opaque ids the
    /// model emits that were never assigned (hallucinated) are left as-is and will
    /// be rejected downstream by the strict action contract (ungrounded ref).
    pub fn deanonymize(&self, value: &Value) -> Value {
        rewrite_value(value, &self.reverse)
    }

    #[cfg(test)]
    pub fn pair_count(&self) -> usize {
        self.forward.len()
    }
}

fn rewrite_value(value: &Value, pairs: &[(String, String)]) -> Value {
    match value {
        Value::String(s) => Value::String(rewrite_str(s, pairs)),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| rewrite_value(v, pairs)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), rewrite_value(v, pairs)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn rewrite_str(value: &str, pairs: &[(String, String)]) -> String {
    let mut out = value.to_string();
    for (from, to) in pairs {
        if out.contains(from.as_str()) {
            out = out.replace(from.as_str(), to);
        }
    }
    out
}

fn collect_domain_ref_values(value: &Value, sink: &mut impl FnMut(&str)) {
    match value {
        Value::String(s) if looks_like_domain_ref(s) => sink(s),
        Value::Array(items) => items
            .iter()
            .for_each(|v| collect_domain_ref_values(v, sink)),
        Value::Object(map) => map
            .values()
            .for_each(|v| collect_domain_ref_values(v, sink)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trips_refs_and_rewrites_the_goal() {
        let about = "about:incident:checkout-latency:case-1";
        let node = "about:incident:checkout-latency:case-1:node:symptom:000";
        let known = vec![node.to_string()];
        let subject = json!({
            "about": about,
            "goal": format!("Inspect node {node} to read its metadata."),
            "visible_state": {
                "known_refs": [node],
                "known_dimensions": ["agent:reviewer", "topic:infra"],
            },
        });
        let map = RefAnonymization::build(about, &known, &subject);
        let anon = map.anonymize(&subject);

        // scope -> about_0001, node -> ref_0001
        assert_eq!(anon["about"], json!("about_0001"));
        assert_eq!(anon["visible_state"]["known_refs"], json!(["ref_0001"]));
        // goal had the node ref substituted, not the scope prefix
        assert_eq!(
            anon["goal"],
            json!("Inspect node ref_0001 to read its metadata.")
        );
        // dimensions are structural and preserved
        assert_eq!(
            anon["visible_state"]["known_dimensions"],
            json!(["agent:reviewer", "topic:infra"])
        );
        // de-anonymizing restores the original
        assert_eq!(map.deanonymize(&anon), subject);
    }

    #[test]
    fn deanonymizes_an_action_back_to_real_refs() {
        let about = "about:bug:webhook:case-2";
        let node = "about:bug:webhook:case-2:node:trace:001";
        let map = RefAnonymization::build(
            about,
            &[node.to_string()],
            &json!({"about": about, "visible_state": {"known_refs": [node]}}),
        );
        // model emitted an action referencing the opaque ref
        let action = json!({"arguments": {"target": "ref_0001"}, "kind": "tool_call", "tool": "kernel_inspect"});
        let real = map.deanonymize(&action);
        assert_eq!(real["arguments"]["target"], json!(node));
    }

    #[test]
    fn discovers_refs_not_in_known_refs() {
        // a prepared-action ref that is not listed in known_refs is still mapped
        let about = "about:docs:rfc:case-3";
        let stray = "about:docs:rfc:case-3:node:plan:000";
        let subject = json!({
            "about": about,
            "prepared_action": {"action": {"arguments": {"target": stray}}},
            "visible_state": {"known_refs": []},
        });
        let map = RefAnonymization::build(about, &[], &subject);
        assert_eq!(map.pair_count(), 2); // about + stray
        let anon = map.anonymize(&subject);
        assert_eq!(
            anon["prepared_action"]["action"]["arguments"]["target"],
            json!("ref_0001")
        );
    }

    #[test]
    fn leaves_unmapped_opaque_ids_untouched_on_deanonymize() {
        let map = RefAnonymization::build(
            "about:x:case-9",
            &["about:x:case-9:node:a:000".to_string()],
            &json!({"about": "about:x:case-9", "visible_state": {"known_refs": ["about:x:case-9:node:a:000"]}}),
        );
        // ref_0099 was never assigned -> stays (will fail downstream grounding)
        let action = json!({"arguments": {"target": "ref_0099"}});
        assert_eq!(
            map.deanonymize(&action)["arguments"]["target"],
            json!("ref_0099")
        );
    }
}
