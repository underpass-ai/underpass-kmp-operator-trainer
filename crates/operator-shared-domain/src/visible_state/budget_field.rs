#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    Unbounded,
    Bounded(usize),
}
