use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskArguments {
    query: NonEmptyString,
}

impl AskArguments {
    pub fn new(query: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            query: NonEmptyString::parse(query, "ask_arguments.query")?,
        })
    }

    pub fn query(&self) -> &NonEmptyString {
        &self.query
    }
}
