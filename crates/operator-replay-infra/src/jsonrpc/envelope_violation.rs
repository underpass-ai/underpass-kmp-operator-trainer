/// Outcome of envelope validation. Mirrors JSON-RPC 2.0 section 5.1.
#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeViolation {
    WrongJsonRpcVersion { actual: String },
    MissingId,
    IdMismatch { expected: u64, actual: u64 },
    ResultAndErrorBothPresent,
    NeitherResultNorError,
}
