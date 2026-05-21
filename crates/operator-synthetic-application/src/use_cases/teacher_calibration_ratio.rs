//! Accuracy ratio helper for bounded calibration counts.

pub fn accuracy_ratio(matches: usize, total: usize) -> Option<f64> {
    if total == 0 {
        return None;
    }
    Some(to_f64(matches) / to_f64(total))
}

fn to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).expect("calibration count exceeds u32"))
}
