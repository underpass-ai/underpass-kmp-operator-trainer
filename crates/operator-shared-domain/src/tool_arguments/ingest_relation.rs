use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::tool_arguments::ingest_confidence::IngestConfidence;
use crate::tool_arguments::ingest_relation_class::IngestRelationClass;
use crate::tool_arguments::ingest_relation_type::IngestRelationType;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRelation {
    source_ref: MemoryRef,
    target_ref: MemoryRef,
    relation_type: IngestRelationType,
    semantic_class: IngestRelationClass,
    why: Option<NonEmptyString>,
    evidence: Option<NonEmptyString>,
    confidence: Option<IngestConfidence>,
    sequence: Option<PositiveCount>,
}

impl IngestRelation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_ref: MemoryRef,
        target_ref: MemoryRef,
        relation_type: IngestRelationType,
        semantic_class: IngestRelationClass,
        why: Option<NonEmptyString>,
        evidence: Option<NonEmptyString>,
        confidence: Option<IngestConfidence>,
        sequence: Option<PositiveCount>,
    ) -> DomainResult<Self> {
        if !semantic_class.is_structural() {
            if confidence.is_none() {
                return Err(DomainError::TrajectoryInvariantMismatch {
                    field: "ingest.memory.relations[].confidence",
                    expected: "non-structural relations require confidence".to_string(),
                    actual: "missing".to_string(),
                });
            }
            if why.is_none() && evidence.is_none() {
                return Err(DomainError::TrajectoryInvariantMismatch {
                    field: "ingest.memory.relations[].why_or_evidence",
                    expected: "non-structural relations require why or evidence".to_string(),
                    actual: "missing".to_string(),
                });
            }
        }
        Ok(Self {
            source_ref,
            target_ref,
            relation_type,
            semantic_class,
            why,
            evidence,
            confidence,
            sequence,
        })
    }

    pub fn source_ref(&self) -> &MemoryRef {
        &self.source_ref
    }

    pub fn target_ref(&self) -> &MemoryRef {
        &self.target_ref
    }

    pub fn relation_type(&self) -> &IngestRelationType {
        &self.relation_type
    }

    pub fn semantic_class(&self) -> IngestRelationClass {
        self.semantic_class
    }

    pub fn why(&self) -> Option<&NonEmptyString> {
        self.why.as_ref()
    }

    pub fn evidence(&self) -> Option<&NonEmptyString> {
        self.evidence.as_ref()
    }

    pub fn confidence(&self) -> Option<IngestConfidence> {
        self.confidence
    }

    pub fn sequence(&self) -> Option<PositiveCount> {
        self.sequence
    }
}
