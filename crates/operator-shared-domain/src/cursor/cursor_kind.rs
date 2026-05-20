/// Discriminator-only view of `Cursor`, useful for routing and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CursorKind {
    Ref,
    Around,
    Temporal,
    Trace,
}

impl CursorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ref => "ref",
            Self::Around => "around",
            Self::Temporal => "temporal",
            Self::Trace => "trace",
        }
    }
}
