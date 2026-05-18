use crate::cursor::cursor::Cursor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GotoArguments {
    cursor: Cursor,
}

impl GotoArguments {
    pub fn new(cursor: Cursor) -> Self {
        Self { cursor }
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }
}
