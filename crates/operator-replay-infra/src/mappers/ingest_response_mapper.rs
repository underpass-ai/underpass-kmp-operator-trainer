use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_outcomes::ingest_outcome::IngestOutcome;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use serde_json::Value;

use crate::mappers::mapping_error::MappingError;

const TOOL: &str = "ingest";

#[derive(Debug)]
pub struct IngestResponseMapper;

impl IngestResponseMapper {
    pub fn to_outcome(structured: &Value) -> Result<IngestOutcome, MappingError> {
        let summary = required_non_empty(structured, "summary", "ingest_response.summary")?;
        let memory = structured.get("memory").and_then(Value::as_object).ok_or(
            MappingError::MissingField {
                tool: TOOL,
                field: "memory",
            },
        )?;
        let about = memory
            .get("about")
            .and_then(Value::as_str)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "memory.about",
            })
            .and_then(|raw| {
                AboutId::parse(raw).map_err(|err| MappingError::InvalidValue {
                    tool: TOOL,
                    field: "memory.about",
                    message: err.to_string(),
                })
            })?;
        let memory_id = memory
            .get("memory_id")
            .and_then(Value::as_str)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "memory.memory_id",
            })
            .and_then(|raw| {
                NonEmptyString::parse(raw, "ingest_response.memory_id").map_err(|err| {
                    MappingError::InvalidValue {
                        tool: TOOL,
                        field: "memory.memory_id",
                        message: err.to_string(),
                    }
                })
            })?;
        let read_after_write_ready = memory
            .get("read_after_write_ready")
            .and_then(Value::as_bool)
            .ok_or(MappingError::MissingField {
                tool: TOOL,
                field: "memory.read_after_write_ready",
            })?;
        let warnings = warnings(structured)?;
        Ok(IngestOutcome::new(
            summary,
            about,
            memory_id,
            read_after_write_ready,
            warnings,
        ))
    }
}

fn required_non_empty(
    structured: &Value,
    field: &'static str,
    context: &'static str,
) -> Result<NonEmptyString, MappingError> {
    let raw = structured
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MappingError::MissingField { tool: TOOL, field })?;
    NonEmptyString::parse(raw, context).map_err(|err| MappingError::InvalidValue {
        tool: TOOL,
        field,
        message: err.to_string(),
    })
}

fn warnings(structured: &Value) -> Result<Vec<NonEmptyString>, MappingError> {
    let Some(raw) = structured.get("warnings") else {
        return Ok(Vec::new());
    };
    let array = raw.as_array().ok_or(MappingError::WrongType {
        tool: TOOL,
        field: "warnings",
        expected: "array",
    })?;
    let mut out = Vec::with_capacity(array.len());
    for item in array {
        let raw = item.as_str().ok_or(MappingError::WrongType {
            tool: TOOL,
            field: "warnings[]",
            expected: "string",
        })?;
        out.push(
            NonEmptyString::parse(raw, "ingest_response.warnings[]").map_err(|err| {
                MappingError::InvalidValue {
                    tool: TOOL,
                    field: "warnings[]",
                    message: err.to_string(),
                }
            })?,
        );
    }
    Ok(out)
}
