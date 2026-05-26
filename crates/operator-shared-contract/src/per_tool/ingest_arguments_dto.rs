use serde::{Deserialize, Serialize};

use crate::per_tool::ingest_memory_dto::IngestMemoryDto;
use crate::per_tool::ingest_provenance_dto::IngestProvenanceDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestArgumentsDto {
    pub about: String,
    pub memory: IngestMemoryDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<IngestProvenanceDto>,
    pub idempotency_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
