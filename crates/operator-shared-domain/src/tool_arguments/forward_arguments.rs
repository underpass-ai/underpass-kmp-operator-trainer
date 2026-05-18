use crate::cursor::temporal_cursor::TemporalCursor;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardArguments {
    cursor: TemporalCursor,
    window: PositiveCount,
}

impl ForwardArguments {
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
