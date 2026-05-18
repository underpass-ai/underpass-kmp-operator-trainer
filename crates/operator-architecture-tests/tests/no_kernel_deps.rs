//! ADR 0001 + principle 9: no Operator crate may depend on a
//! `rehydration-*` (kernel) crate. Enforced by manifest scan.

use operator_architecture_tests::crate_inventory::operator_crates;

#[test]
fn no_operator_crate_imports_rehydration_kernel() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        let text = op.manifest_text();
        if text.contains("rehydration-") {
            offenders.push(op.name);
        }
    }
    assert!(
        offenders.is_empty(),
        "the following Operator crates contain a forbidden `rehydration-` dependency in their Cargo.toml: {offenders:?}"
    );
}
