//! ADR 0002: every `src/*.rs` file declares at most one public
//! `struct`, `enum` or `trait`.

use std::fs;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::source_walker::rust_files;

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
            offenders.push(format!("{} ({} public types)", path.display(), count));
        }
    }
    assert!(
        offenders.is_empty(),
        "files declaring more than one public top-level type: {offenders:?}"
    );
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
