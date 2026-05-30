//! Wire form of one cross-about target: an about and its gold period entries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossAboutTargetDto {
    pub about: String,
    /// Gold set of period entry refs in this about. Cross-about coverage is
    /// gold-only, so the mapper rejects a target with no gold.
    #[serde(default)]
    pub expected_refs: Vec<String>,
}
