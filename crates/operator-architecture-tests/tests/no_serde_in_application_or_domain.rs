//! Principle 1 + 7: `serde` and `serde_json` are infrastructure
//! concerns. `*-domain` and `*-application` crates must not declare
//! either dependency. The complementary `serde_json::Value` source-level
//! check lives in `no_serde_json_in_domain_or_application.rs`.

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::crate_kind::CrateKind;

#[test]
fn domain_and_application_manifests_must_not_depend_on_serde() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        if !matches!(op.kind, CrateKind::Domain | CrateKind::Application) {
            continue;
        }
        let text = op.manifest_text();
        // Look for the dependency line shape `serde.workspace = true` or
        // `serde = "..."`. Avoid false-positives on serde_json by matching
        // a boundary character after the name.
        let lines: Vec<&str> = text.lines().collect();
        for line in &lines {
            let trimmed = line.trim_start();
            if trimmed.starts_with("serde.workspace")
                || trimmed.starts_with("serde =")
                || trimmed.starts_with("serde={")
                || trimmed.starts_with("serde =\"")
            {
                offenders.push(op.name.clone());
                break;
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "domain/application crates with forbidden `serde` dependency: {offenders:?}"
    );
}
