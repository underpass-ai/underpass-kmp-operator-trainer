/// Layer kind of an Operator crate, inferred from its name suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateKind {
    Contract,
    Domain,
    Application,
    Infra,
    Cli,
    Tests,
    Unknown,
}

impl CrateKind {
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        if name.ends_with("-contract") {
            Self::Contract
        } else if name.ends_with("-domain") {
            Self::Domain
        } else if name.ends_with("-application") {
            Self::Application
        } else if name.ends_with("-infra") {
            Self::Infra
        } else if name.ends_with("-cli") {
            Self::Cli
        } else if name.ends_with("-tests") {
            Self::Tests
        } else {
            Self::Unknown
        }
    }
}
