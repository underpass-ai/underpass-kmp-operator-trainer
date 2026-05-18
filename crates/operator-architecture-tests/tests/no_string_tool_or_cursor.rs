//! ADR 0003: tools and cursors must be typed. `tool: &str` and
//! `cursor_key: &str` are banned everywhere — including infra mappers,
//! which decode them via typed `KernelTool::parse` / `TemporalCursorKey::parse`
//! rather than carrying them as raw strings.

use std::fs;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::source_walker::rust_files;

#[test]
fn no_source_file_contains_tool_or_cursor_key_string_parameter() {
    let mut offenders = Vec::new();
    for op in operator_crates() {
        for path in rust_files(&op.src_dir) {
            let text = fs::read_to_string(&path).unwrap_or_default();
            if text.contains("tool: &str") || text.contains("cursor_key: &str") {
                offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "files with primitive-string tool/cursor_key parameters: {offenders:?}"
    );
}
