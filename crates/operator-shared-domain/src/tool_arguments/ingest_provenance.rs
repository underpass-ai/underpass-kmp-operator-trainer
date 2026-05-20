use std::time::SystemTime;

use crate::tool_arguments::ingest_source_kind::IngestSourceKind;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestProvenance {
    source_kind: IngestSourceKind,
    source_agent: NonEmptyString,
    observed_at: SystemTime,
    correlation_id: Option<NonEmptyString>,
    causation_id: Option<NonEmptyString>,
}

impl IngestProvenance {
    pub fn new(
        source_kind: IngestSourceKind,
        source_agent: NonEmptyString,
        observed_at: SystemTime,
        correlation_id: Option<NonEmptyString>,
        causation_id: Option<NonEmptyString>,
    ) -> Self {
        Self {
            source_kind,
            source_agent,
            observed_at,
            correlation_id,
            causation_id,
        }
    }

    pub fn source_kind(&self) -> IngestSourceKind {
        self.source_kind
    }

    pub fn source_agent(&self) -> &NonEmptyString {
        &self.source_agent
    }

    pub fn observed_at(&self) -> SystemTime {
        self.observed_at
    }

    pub fn correlation_id(&self) -> Option<&NonEmptyString> {
        self.correlation_id.as_ref()
    }

    pub fn causation_id(&self) -> Option<&NonEmptyString> {
        self.causation_id.as_ref()
    }
}
