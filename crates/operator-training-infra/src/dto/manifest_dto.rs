//! Top-level TOML shape of a `TrainingManifest`. Per ADR 0012 §4.

use serde::Serialize;

use crate::dto::manifest_dataset_dto::ManifestDatasetDto;
use crate::dto::manifest_readiness_dto::ManifestReadinessDto;
use crate::dto::manifest_run_dto::ManifestRunDto;
use crate::dto::manifest_trainer_target_dto::ManifestTrainerTargetDto;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestDto {
    pub run: ManifestRunDto,
    pub dataset: ManifestDatasetDto,
    pub readiness: ManifestReadinessDto,
    pub trainer_target: ManifestTrainerTargetDto,
}
