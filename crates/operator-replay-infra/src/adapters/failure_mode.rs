#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    Transport,
    Protocol,
    InvalidArguments,
    MalformedResponse,
}
