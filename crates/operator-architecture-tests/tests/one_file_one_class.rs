//! ADR 0002: every `src/*.rs` file declares at most one public
//! `struct`, `enum` or `trait`. Exceptions are listed in
//! `KNOWN_EXCEPTIONS` with a justification comment.

use std::fs;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::source_walker::rust_files;

/// Files allowed to declare more than one public type, together with the
/// reason. Add an entry only with a justification.
const KNOWN_EXCEPTIONS: &[(&str, &str)] = &[
    (
        "cursor/cursor.rs",
        "Cursor enum + CursorKind discriminator are intrinsically paired",
    ),
    (
        "action/operator_action.rs",
        "OperatorAction enum + OperatorActionKind discriminator are intrinsically paired",
    ),
    (
        "errors/infra_error.rs",
        "InfraError enum + thiserror `From` impl conversions",
    ),
    (
        "visible_state/budget_snapshot.rs",
        "BudgetSnapshot struct + BudgetField enum are intrinsically paired",
    ),
    (
        "visible_state_dto.rs",
        "VisibleStateDto + BudgetSnapshotDto are intrinsically paired wire types",
    ),
    (
        "adapters/in_memory_kmp_mcp_client.rs",
        "InMemoryKmpMcpClient + FailureMode config enum are intrinsically paired test-fixture types",
    ),
];

#[test]
fn each_source_file_declares_at_most_one_public_type() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        for path in rust_files(&op.src_dir) {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let count = count_public_top_level_types(&text);
            if count <= 1 {
                continue;
            }
            let suffix = relative_under_src(&path);
            let allowed = KNOWN_EXCEPTIONS.iter().any(|(s, _)| suffix.ends_with(s));
            if !allowed {
                offenders.push(format!("{} ({} public types)", path.display(), count));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "files declaring more than one public top-level type: {offenders:?}"
    );
}

fn relative_under_src(path: &std::path::Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut hit_src = false;
    for component in path.components() {
        let segment = component.as_os_str().to_string_lossy().into_owned();
        if segment == "src" {
            hit_src = true;
            continue;
        }
        if hit_src {
            parts.push(segment);
        }
    }
    parts.join("/")
}

fn count_public_top_level_types(text: &str) -> usize {
    let mut count = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("pub ") {
            continue;
        }
        let rest = &trimmed["pub ".len()..];
        if rest.starts_with("struct ") || rest.starts_with("enum ") || rest.starts_with("trait ") {
            count += 1;
        }
    }
    count
}
