//! Sanity check: every Operator crate listed in the workspace exists,
//! and every directory under `crates/` is registered as a workspace
//! member. This is meta-test that prevents silent drift between the
//! workspace manifest and the on-disk crates.

use std::fs;
use std::path::PathBuf;

use operator_architecture_tests::crate_inventory::operator_crates;
use operator_architecture_tests::workspace_root::workspace_root;

#[test]
fn workspace_members_match_crates_dir() {
    let manifest = fs::read_to_string(workspace_root().join("Cargo.toml"))
        .expect("workspace Cargo.toml readable");
    let registered: Vec<String> = manifest
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let trimmed = trimmed.strip_prefix('"')?.strip_suffix("\",")?;
            Some(trimmed.to_string())
        })
        .filter(|s| s.starts_with("crates/"))
        .collect();

    let on_disk: Vec<String> = operator_crates()
        .into_iter()
        .map(|c| {
            PathBuf::from("crates")
                .join(&c.name)
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    for expected in &on_disk {
        assert!(
            registered.contains(expected),
            "{expected} exists on disk but is not in workspace members"
        );
    }
    for declared in &registered {
        assert!(
            on_disk.contains(declared),
            "{declared} is declared in workspace but missing on disk"
        );
    }
}
