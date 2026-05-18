//! TOML `[readiness]` section: the overall verdict plus the per-gate
//! checks as a `[[readiness.gate]]` array.

use serde::Serialize;

use crate::dto::manifest_readiness_gate_dto::ManifestReadinessGateDto;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestReadinessDto {
    pub overall: &'static str,
    pub gate: Vec<ManifestReadinessGateDto>,
}
