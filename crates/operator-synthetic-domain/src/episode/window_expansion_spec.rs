//! Parameters of a window-expansion episode.
//!
//! A window-expansion episode asks the operator to answer a count-over-period
//! style question that does not fit one bounded read: it opens a bounded wake
//! window (`budget.max_entries`, so the kernel surfaces only the first N entries
//! and reports the rest via `proof.frontier_size`) and then widens coverage with
//! `kernel_near` / `kernel_forward` / `kernel_rewind` across several calls until
//! the period's evidence is fully covered. This value object only
//! *parameterizes* the scenario — the bounded wake window and how many expansion
//! steps the loop is allowed to take. It deliberately holds **no** expansion
//! schedule: deciding how far to widen each step, and when coverage is complete,
//! stays the policy's job so it remains learnable. The deterministic
//! verification of whether coverage was actually reached lives in the
//! window-expansion oracle, never here.

use operator_shared_domain::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowExpansionSpec {
    initial_window: PositiveCount,
    max_iterations: PositiveCount,
}

impl WindowExpansionSpec {
    pub fn new(initial_window: PositiveCount, max_iterations: PositiveCount) -> Self {
        Self {
            initial_window,
            max_iterations,
        }
    }

    /// Size of the bounded wake window the scenario imposes: the operator's
    /// first read wakes with `budget.max_entries = initial_window`, so the
    /// kernel surfaces only this many entries and reports the rest as
    /// `proof.frontier_size`, creating the tail the policy must expand into.
    pub fn initial_window(&self) -> PositiveCount {
        self.initial_window
    }

    /// Upper bound on expansion tool calls the session may spend before the
    /// policy must terminate. Bounds the loop so a never-satisfied policy
    /// cannot run unbounded.
    pub fn max_iterations(&self) -> PositiveCount {
        self.max_iterations
    }

    /// Call budget a session needs: one call per allowed expansion step plus a
    /// final call for the terminal stop/escalate decision. Saturates at
    /// [`u32::MAX`] rather than overflowing.
    pub fn required_calls(&self) -> u32 {
        let iterations = u32::try_from(self.max_iterations.as_usize()).unwrap_or(u32::MAX);
        iterations.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(value: usize) -> PositiveCount {
        PositiveCount::parse(value, "test").expect("positive count")
    }

    #[test]
    fn exposes_parts() {
        let spec = WindowExpansionSpec::new(count(8), count(5));
        assert_eq!(spec.initial_window().as_usize(), 8);
        assert_eq!(spec.max_iterations().as_usize(), 5);
        assert_eq!(spec, spec);
    }

    #[test]
    fn required_calls_reserves_one_for_the_terminal_decision() {
        let spec = WindowExpansionSpec::new(count(2), count(5));
        assert_eq!(spec.required_calls(), 6);
    }
}
