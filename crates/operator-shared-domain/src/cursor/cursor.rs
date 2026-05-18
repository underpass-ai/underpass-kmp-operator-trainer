//! `Cursor` is the closed enum of operator-facing cursors. The variant is
//! the kind of cursor; the variant payload is the typed cursor body.

use crate::cursor::around_cursor::AroundCursor;
use crate::cursor::ref_cursor::RefCursor;
use crate::cursor::temporal_cursor::TemporalCursor;
use crate::cursor::trace_cursor::TraceCursor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    Ref(RefCursor),
    Around(AroundCursor),
    Temporal(TemporalCursor),
    Trace(TraceCursor),
}

impl Cursor {
    pub fn kind(&self) -> CursorKind {
        match self {
            Self::Ref(_) => CursorKind::Ref,
            Self::Around(_) => CursorKind::Around,
            Self::Temporal(_) => CursorKind::Temporal,
            Self::Trace(_) => CursorKind::Trace,
        }
    }
}

/// Discriminator-only view of `Cursor`, useful for routing and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CursorKind {
    Ref,
    Around,
    Temporal,
    Trace,
}

impl CursorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ref => "ref",
            Self::Around => "around",
            Self::Temporal => "temporal",
            Self::Trace => "trace",
        }
    }
}
