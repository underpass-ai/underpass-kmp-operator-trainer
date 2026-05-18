use std::path::PathBuf;

/// Absolute path to the workspace root, computed from this crate's
/// `CARGO_MANIFEST_DIR` env var. Cargo guarantees this var at compile time
/// and is stable across cargo invocations within the same workspace.
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has parent")
        .parent()
        .expect("crates dir has parent")
        .to_path_buf()
}

/// Returns the `crates/` directory inside the workspace root.
#[must_use]
pub fn crates_dir() -> PathBuf {
    workspace_root().join("crates")
}
