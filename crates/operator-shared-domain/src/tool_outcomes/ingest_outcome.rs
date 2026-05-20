use crate::ids::about_id::AboutId;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestOutcome {
    summary: NonEmptyString,
    about: AboutId,
    memory_id: NonEmptyString,
    read_after_write_ready: bool,
    warnings: Vec<NonEmptyString>,
}

impl IngestOutcome {
    pub fn new(
        summary: NonEmptyString,
        about: AboutId,
        memory_id: NonEmptyString,
        read_after_write_ready: bool,
        warnings: Vec<NonEmptyString>,
    ) -> Self {
        Self {
            summary,
            about,
            memory_id,
            read_after_write_ready,
            warnings,
        }
    }

    pub fn summary(&self) -> &NonEmptyString {
        &self.summary
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn memory_id(&self) -> &NonEmptyString {
        &self.memory_id
    }

    pub fn read_after_write_ready(&self) -> bool {
        self.read_after_write_ready
    }

    pub fn warnings(&self) -> &[NonEmptyString] {
        &self.warnings
    }
}
