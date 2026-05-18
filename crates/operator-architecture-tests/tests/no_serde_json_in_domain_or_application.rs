//! ADR 0004 + principle 7: `serde_json` is forbidden in `*-domain` and
//! `*-application` crates. Enforced by manifest scan AND source scan
//! (catches inline references such as `serde_json::Value` or `json!(`).

use std::fs;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::crate_kind::CrateKind;
use operator_architecture_tests::source_walker::rust_files;

#[test]
fn domain_and_application_manifests_must_not_depend_on_serde_json() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        if !matches!(op.kind, CrateKind::Domain | CrateKind::Application) {
            continue;
        }
        let text = op.manifest_text();
        if text.contains("serde_json") {
            offenders.push(op.name);
        }
    }
    assert!(
        offenders.is_empty(),
        "domain/application crates with forbidden `serde_json` dependency: {offenders:?}"
    );
}

#[test]
fn domain_and_application_sources_must_not_reference_serde_json_value_or_json_macro() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        if !matches!(op.kind, CrateKind::Domain | CrateKind::Application) {
            continue;
        }
        for path in rust_files(&op.src_dir) {
            let text = fs::read_to_string(&path).unwrap_or_default();
            if text.contains("serde_json::Value") || text.contains("json!(") {
                offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "domain/application files reference serde_json::Value or json!(...): {offenders:?}"
    );
}
