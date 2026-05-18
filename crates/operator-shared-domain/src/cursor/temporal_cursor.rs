use crate::cursor::temporal_anchor::TemporalAnchor;
use crate::cursor::temporal_cursor_key::TemporalCursorKey;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporalCursor {
    key: TemporalCursorKey,
    anchor: TemporalAnchor,
}

impl TemporalCursor {
    pub fn new(key: TemporalCursorKey, anchor: TemporalAnchor) -> Self {
        Self { key, anchor }
    }

    pub fn key(&self) -> TemporalCursorKey {
        self.key
    }

    pub fn anchor(&self) -> &TemporalAnchor {
        &self.anchor
    }
}
