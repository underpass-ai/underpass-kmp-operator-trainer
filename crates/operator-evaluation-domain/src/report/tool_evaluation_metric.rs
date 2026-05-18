//! Per-`KernelTool` evaluation metric. Counts are over outcomes whose
//! ground truth was a `ToolCall` with the given tool. Stop/Escalate
//! outcomes are aggregated separately by the report.

use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolEvaluationMetric {
    tool: Option<KernelTool>,
    total: usize,
    exact_matches: usize,
    tool_matches: usize,
    contract_valid: usize,
}

impl ToolEvaluationMetric {
    /// Creates an empty metric for the given tool. `tool == None` is the
    /// bucket for actions without a kernel tool (Stop / Escalate).
    pub fn empty_for(tool: Option<KernelTool>) -> Self {
        Self {
            tool,
            ..Self::default()
        }
    }

    pub fn record(&mut self, is_exact: bool, is_tool: bool, is_contract_valid: bool) {
        self.total += 1;
        if is_exact {
            self.exact_matches += 1;
        }
        if is_tool {
            self.tool_matches += 1;
        }
        if is_contract_valid {
            self.contract_valid += 1;
        }
    }

    pub fn tool(&self) -> Option<KernelTool> {
        self.tool
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn exact_matches(&self) -> usize {
        self.exact_matches
    }

    pub fn tool_matches(&self) -> usize {
        self.tool_matches
    }

    pub fn contract_valid(&self) -> usize {
        self.contract_valid
    }

    pub fn exact_match_rate(&self) -> f64 {
        rate(self.exact_matches, self.total)
    }

    pub fn tool_match_rate(&self) -> f64 {
        rate(self.tool_matches, self.total)
    }

    pub fn contract_validity_rate(&self) -> f64 {
        rate(self.contract_valid, self.total)
    }
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        let value = numerator as f64 / denominator as f64;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_metric_has_zero_rates() {
        let metric = ToolEvaluationMetric::empty_for(Some(KernelTool::Inspect));
        assert!(metric.exact_match_rate().abs() < f64::EPSILON);
        assert!(metric.tool_match_rate().abs() < f64::EPSILON);
        assert!(metric.contract_validity_rate().abs() < f64::EPSILON);
    }

    #[test]
    fn record_accumulates_counts_and_rates() {
        let mut metric = ToolEvaluationMetric::empty_for(Some(KernelTool::Inspect));
        metric.record(true, true, true);
        metric.record(false, true, true);
        metric.record(false, false, false);

        assert_eq!(metric.total(), 3);
        assert_eq!(metric.exact_matches(), 1);
        assert_eq!(metric.tool_matches(), 2);
        assert_eq!(metric.contract_valid(), 2);

        assert!((metric.exact_match_rate() - 1.0 / 3.0).abs() < f64::EPSILON);
        assert!((metric.tool_match_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
        assert!((metric.contract_validity_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
    }
}
