//! Response envelope received from an MCP server for a `tools/call`
//! RPC. Either `result` is present and carries the tool's
//! structured content, or `error` is present and indicates a JSON-RPC
//! protocol-level failure.
//!
//! The MCP spec exposes the tool payload at either
//! `result.structuredContent` (modern) or
//! `result.content[0].text` as a JSON-encoded string (legacy). This
//! module accepts both shapes.
//!
//! Envelope validation enforced here (per JSON-RPC 2.0 §5.1):
//! - `jsonrpc` field must be present and equal to `"2.0"`.
//! - `id` field must be present and numeric.
//! - Exactly one of `result` / `error` must be present.

use serde::Deserialize;
use serde_json::Value;

use crate::jsonrpc::json_rpc_error::JsonRpcError;

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResponse {
    #[serde(default)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<ToolsCallResult>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResult {
    /// Newer MCP transport ships the typed payload here directly.
    #[serde(default, rename = "structuredContent")]
    pub structured_content: Option<Value>,
    /// Older MCP transport carries a `content` array; the first entry
    /// has `type: "text"` and a JSON-encoded string in `text`.
    #[serde(default)]
    pub content: Vec<ToolsCallResultContent>,
    /// Some MCP versions also expose `isError`; treat true as a tool-
    /// reported failure that the adapter surfaces as a protocol error.
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallResultContent {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

/// Outcome of envelope validation. Mirrors JSON-RPC 2.0 §5.1.
#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeViolation {
    WrongJsonRpcVersion { actual: String },
    MissingId,
    IdMismatch { expected: u64, actual: u64 },
    ResultAndErrorBothPresent,
    NeitherResultNorError,
}

impl ToolsCallResponse {
    /// Validate the envelope against JSON-RPC 2.0 §5.1, matching the
    /// id against the request id. Called once after deserialization,
    /// before extracting the structured payload.
    pub fn validate(&self, expected_id: u64) -> Result<(), EnvelopeViolation> {
        if self.jsonrpc != "2.0" {
            return Err(EnvelopeViolation::WrongJsonRpcVersion {
                actual: self.jsonrpc.clone(),
            });
        }
        let Some(id) = self.id else {
            return Err(EnvelopeViolation::MissingId);
        };
        if id != expected_id {
            return Err(EnvelopeViolation::IdMismatch {
                expected: expected_id,
                actual: id,
            });
        }
        match (self.result.is_some(), self.error.is_some()) {
            (true, true) => Err(EnvelopeViolation::ResultAndErrorBothPresent),
            (false, false) => Err(EnvelopeViolation::NeitherResultNorError),
            _ => Ok(()),
        }
    }

    /// Returns the structured content of a successful response, taking
    /// either the modern `structuredContent` field or the legacy
    /// `content[0].text` JSON-encoded payload. Returns the body
    /// description as an error string if the response is malformed.
    /// Assumes `validate` has already succeeded; if `result` is `None`
    /// this returns an error rather than panicking.
    pub fn structured_content(&self) -> Result<Value, String> {
        let Some(result) = self.result.as_ref() else {
            return Err("response has no result".to_string());
        };
        if result.is_error {
            return Err("server reported is_error = true".to_string());
        }
        if let Some(value) = result.structured_content.as_ref() {
            return Ok(value.clone());
        }
        let first = result
            .content
            .first()
            .ok_or_else(|| "response result missing structuredContent and content".to_string())?;
        if first.kind != "text" {
            return Err(format!(
                "response content[0].type is '{}', expected 'text'",
                first.kind
            ));
        }
        serde_json::from_str(&first.text)
            .map_err(|e| format!("response content[0].text is not JSON: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(json: &str) -> ToolsCallResponse {
        serde_json::from_str(json).expect("envelope parses")
    }

    #[test]
    fn rejects_wrong_jsonrpc_version() {
        let env = envelope(r#"{"jsonrpc":"1.0","id":1,"result":{}}"#);
        assert!(matches!(
            env.validate(1),
            Err(EnvelopeViolation::WrongJsonRpcVersion { .. })
        ));
    }

    #[test]
    fn rejects_missing_id() {
        let env = envelope(r#"{"jsonrpc":"2.0","result":{}}"#);
        assert_eq!(env.validate(1).unwrap_err(), EnvelopeViolation::MissingId);
    }

    #[test]
    fn rejects_id_mismatch() {
        let env = envelope(r#"{"jsonrpc":"2.0","id":7,"result":{}}"#);
        assert_eq!(
            env.validate(1).unwrap_err(),
            EnvelopeViolation::IdMismatch {
                expected: 1,
                actual: 7
            },
        );
    }

    #[test]
    fn rejects_result_and_error_both_present() {
        let env =
            envelope(r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":-1,"message":"x"}}"#);
        assert_eq!(
            env.validate(1).unwrap_err(),
            EnvelopeViolation::ResultAndErrorBothPresent,
        );
    }

    #[test]
    fn rejects_neither_result_nor_error() {
        let env = envelope(r#"{"jsonrpc":"2.0","id":1}"#);
        assert_eq!(
            env.validate(1).unwrap_err(),
            EnvelopeViolation::NeitherResultNorError,
        );
    }

    #[test]
    fn accepts_well_formed_success() {
        let env = envelope(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        assert!(env.validate(1).is_ok());
    }

    #[test]
    fn accepts_well_formed_error() {
        let env = envelope(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"x"}}"#);
        assert!(env.validate(1).is_ok());
    }
}
