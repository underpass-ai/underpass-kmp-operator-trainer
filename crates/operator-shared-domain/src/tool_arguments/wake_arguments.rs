use crate::ids::about_id::AboutId;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeArguments {
    about: AboutId,
    /// Opt-in cap on the entries wake surfaces (the kernel's
    /// `budget.max_entries`). When set and the about has more, wake returns a
    /// window and signals the rest via `proof.frontier_size` so the operator
    /// near-expands. `None` = unbounded (the kernel's default).
    window: Option<PositiveCount>,
}

impl WakeArguments {
    pub fn new(about: AboutId) -> Self {
        Self {
            about,
            window: None,
        }
    }

    /// Request a bounded wake window of `window` entries (the rest is reached by
    /// kernel_near-expansion).
    #[must_use]
    pub fn with_window(mut self, window: PositiveCount) -> Self {
        self.window = Some(window);
        self
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn window(&self) -> Option<PositiveCount> {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_the_about_it_was_built_with() {
        let about = AboutId::parse("about:1").unwrap();
        let args = WakeArguments::new(about.clone());
        assert_eq!(args.about(), &about);
    }
}
