use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::string_map::StringMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestDimension {
    id: DimensionRef,
    kind: NonEmptyString,
    title: Option<NonEmptyString>,
    metadata: StringMap,
}

impl IngestDimension {
    pub fn new(
        id: DimensionRef,
        kind: NonEmptyString,
        title: Option<NonEmptyString>,
        metadata: StringMap,
    ) -> Self {
        Self {
            id,
            kind,
            title,
            metadata,
        }
    }

    pub fn id(&self) -> &DimensionRef {
        &self.id
    }

    pub fn kind(&self) -> &NonEmptyString {
        &self.kind
    }

    pub fn title(&self) -> Option<&NonEmptyString> {
        self.title.as_ref()
    }

    pub fn metadata(&self) -> &StringMap {
        &self.metadata
    }
}
