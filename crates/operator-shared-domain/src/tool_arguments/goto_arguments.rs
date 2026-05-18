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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::ref_cursor::RefCursor;
    use crate::value_objects::memory_ref::MemoryRef;

    #[test]
    fn exposes_the_cursor_it_was_built_with() {
        let target = MemoryRef::parse("node:1").unwrap();
        let cursor = Cursor::Ref(RefCursor::new(target));
        let args = GotoArguments::new(cursor.clone());
        assert_eq!(args.cursor(), &cursor);
    }
}
