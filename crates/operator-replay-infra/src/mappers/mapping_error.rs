//! Mapper-side error for translating MCP structured content into the
//! per-tool outcome value objects in `operator-shared-domain`.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MappingError {
    #[error("kernel_{tool} response missing required field '{field}'")]
    MissingField {
        tool: &'static str,
        field: &'static str,
    },

    #[error("kernel_{tool} response field '{field}' has wrong type: expected {expected}")]
    WrongType {
        tool: &'static str,
        field: &'static str,
        expected: &'static str,
    },

    #[error("kernel_{tool} response field '{field}' contains invalid value: {message}")]
    InvalidValue {
        tool: &'static str,
        field: &'static str,
        message: String,
    },
}
