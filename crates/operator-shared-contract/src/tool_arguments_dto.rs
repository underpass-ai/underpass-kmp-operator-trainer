use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tagged-union DTO for tool arguments. The `tool` field carries the
/// canonical KMP tool name (`kernel_wake`, `kernel_near`, ...). The
/// `arguments` field carries the per-tool payload as a structured JSON
/// object; the shape per tool is documented in
/// `docs/architecture/operator/shared/contract/README.md`.
///
/// This DTO is intentionally permissive at the serde level. The strict
/// shape is enforced by the `ToolArgumentsMapper` in
/// `operator-shared-infra`, which produces typed domain `ToolArguments`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolArgumentsDto {
    pub tool: String,
    pub arguments: Value,
}
