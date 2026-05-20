use std::collections::BTreeMap;

use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringMap {
    entries: BTreeMap<NonEmptyString, NonEmptyString>,
}

impl StringMap {
    pub fn new(entries: BTreeMap<NonEmptyString, NonEmptyString>) -> Self {
        Self { entries }
    }

    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn entries(&self) -> &BTreeMap<NonEmptyString, NonEmptyString> {
        &self.entries
    }
}

impl Default for StringMap {
    fn default() -> Self {
        Self::empty()
    }
}
