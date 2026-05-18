use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskFamily {
    inner: NonEmptyString,
}

impl TaskFamily {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "task_family")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for TaskFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}
