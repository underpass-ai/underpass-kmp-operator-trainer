use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::tool_arguments::ingest_dimension::IngestDimension;
use crate::tool_arguments::ingest_entry::IngestEntry;
use crate::tool_arguments::ingest_evidence::IngestEvidence;
use crate::tool_arguments::ingest_relation::IngestRelation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestMemory {
    dimensions: Vec<IngestDimension>,
    entries: Vec<IngestEntry>,
    relations: Vec<IngestRelation>,
    evidence: Vec<IngestEvidence>,
}

impl IngestMemory {
    pub fn new(
        dimensions: Vec<IngestDimension>,
        entries: Vec<IngestEntry>,
        relations: Vec<IngestRelation>,
        evidence: Vec<IngestEvidence>,
    ) -> DomainResult<Self> {
        if entries.is_empty() {
            return Err(DomainError::CountBelowMinimum {
                context: "ingest.memory.entries",
                minimum: 1,
                actual: 0,
            });
        }
        Ok(Self {
            dimensions,
            entries,
            relations,
            evidence,
        })
    }

    pub fn dimensions(&self) -> &[IngestDimension] {
        &self.dimensions
    }

    pub fn entries(&self) -> &[IngestEntry] {
        &self.entries
    }

    pub fn relations(&self) -> &[IngestRelation] {
        &self.relations
    }

    pub fn evidence(&self) -> &[IngestEvidence] {
        &self.evidence
    }
}
