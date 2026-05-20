use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::string_map::StringMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestEntry {
    id: MemoryRef,
    kind: NonEmptyString,
    text: NonEmptyString,
    coordinates: Vec<IngestTemporalCoordinate>,
    metadata: StringMap,
}

impl IngestEntry {
    pub fn new(
        id: MemoryRef,
        kind: NonEmptyString,
        text: NonEmptyString,
        coordinates: Vec<IngestTemporalCoordinate>,
        metadata: StringMap,
    ) -> DomainResult<Self> {
        if coordinates.is_empty() {
            return Err(DomainError::CountBelowMinimum {
                context: "ingest.memory.entries[].coordinates",
                minimum: 1,
                actual: 0,
            });
        }
        Ok(Self {
            id,
            kind,
            text,
            coordinates,
            metadata,
        })
    }

    pub fn id(&self) -> &MemoryRef {
        &self.id
    }

    pub fn kind(&self) -> &NonEmptyString {
        &self.kind
    }

    pub fn text(&self) -> &NonEmptyString {
        &self.text
    }

    pub fn coordinates(&self) -> &[IngestTemporalCoordinate] {
        &self.coordinates
    }

    pub fn metadata(&self) -> &StringMap {
        &self.metadata
    }
}
