#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ExampleCount {
    raw: usize,
}

impl ExampleCount {
    pub fn new(value: usize) -> Self {
        Self { raw: value }
    }

    pub fn as_usize(self) -> usize {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_a_valid_example_count() {
        assert_eq!(ExampleCount::default().as_usize(), 0);
    }

    #[test]
    fn round_trips_a_usize() {
        assert_eq!(ExampleCount::new(7).as_usize(), 7);
    }
}
