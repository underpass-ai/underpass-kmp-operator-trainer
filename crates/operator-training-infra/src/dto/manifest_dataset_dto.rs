//! TOML `[dataset]` section of the manifest: provenance plus the
//! per-task-family distribution as a `BTreeMap` (TOML serialises this
//! as a `[dataset.task_family_distribution]` inline table).

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDatasetDto {
    pub source: String,
    pub content_hash: String,
    pub trajectory_count: usize,
    pub task_family_distribution: BTreeMap<String, usize>,
}
