//! Principles 1 + 6: ports declared in `*-application/src/ports/*.rs`
//! must take and return only typed domain values. `serde_json::Value`,
//! free-form `&str` arguments and `&[u8]` payloads are forbidden in port
//! signatures; the adapter handles those at its own boundary.

use std::fs;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::crate_kind::CrateKind;

const FORBIDDEN_PORT_PATTERNS: &[&str] = &[
    "serde_json::Value",
    "tool: &str",
    "cursor_key: &str",
    "json!",
];

#[test]
fn application_ports_must_not_expose_primitive_or_json_types() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        if op.kind != CrateKind::Application {
            continue;
        }
        let ports_dir = op.src_dir.join("ports");
        if !ports_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&ports_dir).expect("ports dir readable") {
            let entry = entry.expect("entry readable");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap_or_default();
            for pattern in FORBIDDEN_PORT_PATTERNS {
                if text.contains(pattern) {
                    offenders.push(format!("{} contains '{pattern}'", path.display()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "application ports expose forbidden types: {offenders:?}"
    );
}
