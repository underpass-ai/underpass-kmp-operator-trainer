use crate::cursor::temporal_cursor::TemporalCursor;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewindArguments {
    cursor: TemporalCursor,
    window: PositiveCount,
}

impl RewindArguments {
    pub fn new(cursor: TemporalCursor, window: PositiveCount) -> Self {
        Self { cursor, window }
    }

    pub fn cursor(&self) -> &TemporalCursor {
        &self.cursor
    }

    pub fn window(&self) -> PositiveCount {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::temporal_anchor::TemporalAnchor;
    use crate::cursor::temporal_cursor_key::TemporalCursorKey;

    #[test]
    fn exposes_cursor_and_window() {
        let cursor = TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
        );
        let args = RewindArguments::new(cursor.clone(), PositiveCount::parse(3, "window").unwrap());
        assert_eq!(args.cursor(), &cursor);
        assert_eq!(args.window().as_usize(), 3);
    }
}
