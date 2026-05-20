use std::time::SystemTime;

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::positive_count::PositiveCount;
use crate::value_objects::string_map::StringMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestTemporalCoordinate {
    dimension: DimensionRef,
    scope_id: NonEmptyString,
    occurred_at: Option<SystemTime>,
    observed_at: Option<SystemTime>,
    ingested_at: Option<SystemTime>,
    valid_from: Option<SystemTime>,
    valid_until: Option<SystemTime>,
    sequence: Option<PositiveCount>,
    rank: Option<PositiveCount>,
    metadata: StringMap,
}

impl IngestTemporalCoordinate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dimension: DimensionRef,
        scope_id: NonEmptyString,
        occurred_at: Option<SystemTime>,
        observed_at: Option<SystemTime>,
        ingested_at: Option<SystemTime>,
        valid_from: Option<SystemTime>,
        valid_until: Option<SystemTime>,
        sequence: Option<PositiveCount>,
        rank: Option<PositiveCount>,
        metadata: StringMap,
    ) -> DomainResult<Self> {
        if matches!((valid_from, valid_until), (Some(from), Some(until)) if until < from) {
            return Err(DomainError::TrajectoryInvariantMismatch {
                field: "ingest.memory.entries[].coordinates[].valid_until",
                expected: "valid_until >= valid_from".to_string(),
                actual: "valid_until < valid_from".to_string(),
            });
        }
        Ok(Self {
            dimension,
            scope_id,
            occurred_at,
            observed_at,
            ingested_at,
            valid_from,
            valid_until,
            sequence,
            rank,
            metadata,
        })
    }

    pub fn dimension(&self) -> &DimensionRef {
        &self.dimension
    }

    pub fn scope_id(&self) -> &NonEmptyString {
        &self.scope_id
    }

    pub fn occurred_at(&self) -> Option<SystemTime> {
        self.occurred_at
    }

    pub fn observed_at(&self) -> Option<SystemTime> {
        self.observed_at
    }

    pub fn ingested_at(&self) -> Option<SystemTime> {
        self.ingested_at
    }

    pub fn valid_from(&self) -> Option<SystemTime> {
        self.valid_from
    }

    pub fn valid_until(&self) -> Option<SystemTime> {
        self.valid_until
    }

    pub fn sequence(&self) -> Option<PositiveCount> {
        self.sequence
    }

    pub fn rank(&self) -> Option<PositiveCount> {
        self.rank
    }

    pub fn metadata(&self) -> &StringMap {
        &self.metadata
    }
}
