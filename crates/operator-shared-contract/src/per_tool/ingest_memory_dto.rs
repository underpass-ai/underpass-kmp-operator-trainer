use serde::{Deserialize, Serialize};

use crate::per_tool::ingest_dimension_dto::IngestDimensionDto;
use crate::per_tool::ingest_entry_dto::IngestEntryDto;
use crate::per_tool::ingest_evidence_dto::IngestEvidenceDto;
use crate::per_tool::ingest_relation_dto::IngestRelationDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestMemoryDto {
    pub dimensions: Vec<IngestDimensionDto>,
    pub entries: Vec<IngestEntryDto>,
    #[serde(default)]
    pub relations: Vec<IngestRelationDto>,
    #[serde(default)]
    pub evidence: Vec<IngestEvidenceDto>,
}
