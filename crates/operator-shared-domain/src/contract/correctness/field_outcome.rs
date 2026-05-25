#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldOutcome {
    Pass,
    Fail { expected: String, actual: String },
    SchemaInvalid { message: String },
}

impl FieldOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
}
