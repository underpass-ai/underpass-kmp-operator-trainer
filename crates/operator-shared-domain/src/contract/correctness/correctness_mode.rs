#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorrectnessMode {
    Exact,
    SchemaValid,
    Permissive,
}

impl CorrectnessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::SchemaValid => "schema_valid",
            Self::Permissive => "permissive",
        }
    }
}
