#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldStats {
    total: usize,
    correct: usize,
}

impl FieldStats {
    pub fn empty() -> Self {
        Self {
            total: 0,
            correct: 0,
        }
    }

    pub fn record(&mut self, is_correct: bool) {
        self.total += 1;
        if is_correct {
            self.correct += 1;
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn correct(&self) -> usize {
        self.correct
    }

    pub fn failures(&self) -> usize {
        self.total - self.correct
    }

    pub fn correctness_rate(&self) -> f64 {
        rate(self.correct, self.total)
    }
}

#[allow(clippy::cast_precision_loss)]
fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
