//! TOML `[trainer_target]` section of the manifest.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestTrainerTargetDto {
    pub command: String,
    pub base_model: String,
    pub output_directory: String,
}
