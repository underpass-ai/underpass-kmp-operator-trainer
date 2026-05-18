//! TOML `[[readiness.gate]]` entry. The target value is rendered
//! stringly so a TOML table can hold gates with heterogeneous target
//! shapes (counts vs. rates) without `untagged` enum acrobatics.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ManifestReadinessGateDto {
    pub kind: &'static str,
    pub target: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
