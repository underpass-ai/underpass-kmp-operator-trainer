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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::domain_error::DomainError;

    #[test]
    fn refuses_empty_query() {
        assert!(matches!(
            AskArguments::new(""),
            Err(DomainError::EmptyValue {
                context: "ask_arguments.query"
            })
        ));
    }

    #[test]
    fn accepts_typical_query() {
        let args = AskArguments::new("who decided X?").unwrap();
        assert_eq!(args.query().as_str(), "who decided X?");
    }
}
