use std::time::SystemTime;

use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::string_map::StringMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestEvidence {
    id: NonEmptyString,
    supports: Vec<MemoryRef>,
    text: NonEmptyString,
    source: Option<NonEmptyString>,
    time: Option<SystemTime>,
    metadata: StringMap,
}

impl IngestEvidence {
    pub fn new(
        id: NonEmptyString,
        supports: Vec<MemoryRef>,
        text: NonEmptyString,
        source: Option<NonEmptyString>,
        time: Option<SystemTime>,
        metadata: StringMap,
    ) -> Self {
        Self {
            id,
            supports,
            text,
            source,
            time,
            metadata,
        }
    }

    pub fn id(&self) -> &NonEmptyString {
        &self.id
    }

    pub fn supports(&self) -> &[MemoryRef] {
        &self.supports
    }

    pub fn text(&self) -> &NonEmptyString {
        &self.text
    }

    pub fn source(&self) -> Option<&NonEmptyString> {
        self.source.as_ref()
    }

    pub fn time(&self) -> Option<SystemTime> {
        self.time
    }

    pub fn metadata(&self) -> &StringMap {
        &self.metadata
    }
}
