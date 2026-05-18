use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CursorDto {
    Ref {
        target: String,
    },
    Around {
        anchor: String,
        dimensions: Vec<String>,
    },
    Temporal {
        key: String,
        anchor: String,
    },
    Trace {
        from: String,
        to: String,
    },
}
