//! Command name (typically a CLI executable on PATH or an absolute
//! path) of the external trainer Operator hands the manifest off to.
//! This domain does not validate that the command exists; the infra
//! adapter does that at invocation time.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrainerCommand {
    inner: NonEmptyString,
}

impl TrainerCommand {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "trainer_command")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for TrainerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}
