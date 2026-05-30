//! Verdict of the window-expansion oracle: how covered a finished session's
//! retrieved context was, judged only from the kernel's own in-band signals
//! (no gold answer). Used to accept or drop a transcript before it becomes SFT
//! data — it is never itself a training target.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCoverageOutcome {
    /// The last temporal move reported full coverage: nothing missing, no
    /// unretrieved graph frontier, and no further page. A stop here is
    /// well-grounded.
    Complete,
    /// The session stalled before full coverage: consecutive expansions stopped
    /// surfacing new refs (saturation). The period is exhausted as far as the
    /// kernel can show, so stopping is still defensible even though some
    /// frontier remains.
    Saturated,
    /// Coverage is still open — the last move reported `remaining` residual
    /// shortfall (missing dimensions plus unretrieved frontier) and the session
    /// had not saturated. A stop here is premature. `remaining` is `0` when no
    /// temporal move was made at all, which is still not covered.
    Incomplete { remaining: u32 },
}

impl WindowCoverageOutcome {
    /// Whether the retrieved context is as covered as the kernel can attain —
    /// either fully covered or saturated. The generator keeps transcripts whose
    /// stop was grounded in one of these; an `Incomplete` stop is dropped as a
    /// premature-stop demonstration.
    pub fn is_covered(&self) -> bool {
        matches!(self, Self::Complete | Self::Saturated)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Saturated => "saturated",
            Self::Incomplete { .. } => "incomplete",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_and_saturated_are_covered() {
        assert!(WindowCoverageOutcome::Complete.is_covered());
        assert!(WindowCoverageOutcome::Saturated.is_covered());
    }

    #[test]
    fn incomplete_is_not_covered() {
        assert!(!WindowCoverageOutcome::Incomplete { remaining: 3 }.is_covered());
        assert_eq!(
            WindowCoverageOutcome::Incomplete { remaining: 3 }.as_str(),
            "incomplete"
        );
    }
}
