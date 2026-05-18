//! Principle 1: `*-domain`, `*-application` and `*-contract` crates must
//! not depend on `tokio`, `reqwest`, `tracing` or any other I/O / runtime
//! crate. Infra and CLI are the only layers allowed to talk to the outside
//! world.

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::crate_kind::CrateKind;

const FORBIDDEN_OUTSIDE_INFRA: &[&str] = &[
    "tokio",
    "tokio-stream",
    "reqwest",
    "tracing",
    "tracing-subscriber",
    "tonic",
    "prost",
];

#[test]
fn domain_application_and_contract_manifests_must_not_depend_on_io_runtimes() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        if !matches!(
            op.kind,
            CrateKind::Domain | CrateKind::Application | CrateKind::Contract
        ) {
            continue;
        }
        let text = op.manifest_text();
        for forbidden in FORBIDDEN_OUTSIDE_INFRA {
            let manifest_patterns = [
                format!("{forbidden}.workspace"),
                format!("{forbidden} ="),
                format!("{forbidden}={{"),
            ];
            for line in text.lines() {
                let trimmed = line.trim_start();
                if manifest_patterns
                    .iter()
                    .any(|pattern| trimmed.starts_with(pattern.as_str()))
                {
                    offenders.push(format!("{} -> {forbidden}", op.name));
                    break;
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "non-infra crates depend on I/O runtimes: {offenders:?}"
    );
}
