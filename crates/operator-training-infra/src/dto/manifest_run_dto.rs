//! TOML `[run]` section of the manifest: the run identifier.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestRunDto {
    pub id: String,
}
