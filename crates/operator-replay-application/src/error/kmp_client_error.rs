//! Error returned by every method of `KmpMcpClient`. The variants are
//! deliberately adapter-agnostic so that the JSON-RPC adapter, an
//! in-memory stub, and a future native-gRPC adapter all map onto the
//! same shape.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KmpClientError {
    /// The transport layer failed — connection refused, request timed
    /// out, TLS handshake broke, etc. The adapter's identifier is
    /// carried so logs can attribute the failure.
    #[error("transport failure in adapter '{adapter}' for {tool}: {message}")]
    Transport {
        adapter: &'static str,
        tool: &'static str,
        message: String,
    },

    /// The transport delivered a response but the JSON-RPC envelope was
    /// malformed (missing `result`, server returned `error`, JSON parse
    /// failed). Distinct from `MalformedResponse` so adapters can map
    /// 4xx/5xx and `error` payloads explicitly.
    #[error("protocol failure for {tool}: {message}")]
    Protocol { tool: &'static str, message: String },

    /// The server reported the arguments are invalid (schema rejection,
    /// missing required field). Adapters surface this when the server
    /// returns a structured argument error.
    #[error("invalid arguments for {tool}: {message}")]
    InvalidArguments { tool: &'static str, message: String },

    /// The response body parsed as JSON but did not match the shape the
    /// mapper expects (missing field, wrong type, unknown enum value).
    /// This is the only variant the in-process mapper can emit on its
    /// own; transport failures bubble up from the network adapter.
    #[error("malformed response for {tool}: {message}")]
    MalformedResponse { tool: &'static str, message: String },
}
