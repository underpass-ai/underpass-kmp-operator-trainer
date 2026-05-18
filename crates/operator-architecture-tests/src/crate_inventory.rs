use std::fs;
use std::path::PathBuf;

use crate::crate_kind::CrateKind;
use crate::workspace_root::crates_dir;

#[derive(Debug, Clone)]
pub struct OperatorCrate {
    pub name: String,
    pub kind: CrateKind,
    pub manifest_path: PathBuf,
    pub src_dir: PathBuf,
}

impl OperatorCrate {
    #[must_use]
    pub fn manifest_text(&self) -> String {
        fs::read_to_string(&self.manifest_path).unwrap_or_else(|_| String::new())
    }
}

/// Lists every Operator crate present under `crates/`, ordered by name.
/// Panics if `crates/` cannot be read.
#[must_use]
pub fn operator_crates() -> Vec<OperatorCrate> {
    let mut out = Vec::new();
    for entry in fs::read_dir(crates_dir()).expect("crates/ readable") {
        let entry = entry.expect("entry readable");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let manifest_path = path.join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }
        out.push(OperatorCrate {
            kind: CrateKind::from_name(&name),
            name,
            manifest_path,
            src_dir: path.join("src"),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}
