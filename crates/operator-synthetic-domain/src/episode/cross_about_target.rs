//! One about a cross-about count session must cover.
//!
//! A cross-about count distributes a determined action across several abouts
//! (each about ≈ one session/source). A target names one such about and the
//! gold set of period entries that fall in it — the operands the session must
//! retrieve from that about. The policy discovers each about's entries itself
//! (wake then widen the window), so a target carries no anchor or schedule:
//! only which about, and which of its entries the coverage check expects.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossAboutTarget {
    about: AboutId,
    expected_refs: Vec<MemoryRef>,
}

impl CrossAboutTarget {
    pub fn new(about: AboutId, expected_refs: Vec<MemoryRef>) -> Self {
        Self {
            about,
            expected_refs,
        }
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    /// The gold set of period entries that fall in this about — the operands the
    /// session must retrieve from it.
    pub fn expected_refs(&self) -> &[MemoryRef] {
        &self.expected_refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_about_and_expected_refs() {
        let target = CrossAboutTarget::new(
            AboutId::parse("about:eu").unwrap(),
            vec![
                MemoryRef::parse("node:eu1").unwrap(),
                MemoryRef::parse("node:eu2").unwrap(),
            ],
        );
        assert_eq!(target.about().as_str(), "about:eu");
        assert_eq!(target.expected_refs().len(), 2);
    }
}
