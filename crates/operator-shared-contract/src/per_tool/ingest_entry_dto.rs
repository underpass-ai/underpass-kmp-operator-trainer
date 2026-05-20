use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::per_tool::ingest_temporal_coordinate_dto::IngestTemporalCoordinateDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestEntryDto {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub coordinates: Vec<IngestTemporalCoordinateDto>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}
