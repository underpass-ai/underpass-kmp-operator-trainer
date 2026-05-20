use crate::ids::about_id::AboutId;
use crate::tool_arguments::ingest_memory::IngestMemory;
use crate::tool_arguments::ingest_provenance::IngestProvenance;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestArguments {
    about: AboutId,
    memory: IngestMemory,
    provenance: Option<IngestProvenance>,
    idempotency_key: NonEmptyString,
    dry_run: bool,
}

impl IngestArguments {
    pub fn new(
        about: AboutId,
        memory: IngestMemory,
        provenance: Option<IngestProvenance>,
        idempotency_key: NonEmptyString,
        dry_run: bool,
    ) -> Self {
        Self {
            about,
            memory,
            provenance,
            idempotency_key,
            dry_run,
        }
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn memory(&self) -> &IngestMemory {
        &self.memory
    }

    pub fn provenance(&self) -> Option<&IngestProvenance> {
        self.provenance.as_ref()
    }

    pub fn idempotency_key(&self) -> &NonEmptyString {
        &self.idempotency_key
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
}
