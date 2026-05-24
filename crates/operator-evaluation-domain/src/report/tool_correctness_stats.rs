use std::collections::BTreeMap;

use operator_shared_domain::contract::correctness::field_path::FieldPath;
use operator_shared_domain::tool::kernel_tool::KernelTool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCorrectnessStats {
    tool: Option<KernelTool>,
    total: usize,
    action_correct: usize,
    field_failures: BTreeMap<FieldPath, usize>,
}

impl ToolCorrectnessStats {
    pub fn empty_for(tool: Option<KernelTool>) -> Self {
        Self {
            tool,
            total: 0,
            action_correct: 0,
            field_failures: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, is_action_correct: bool, failed_fields: &[FieldPath]) {
        self.total += 1;
        if is_action_correct {
            self.action_correct += 1;
        }
        for field in failed_fields {
            *self.field_failures.entry(field.clone()).or_insert(0) += 1;
        }
    }

    pub fn tool(&self) -> Option<KernelTool> {
        self.tool
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn action_correct(&self) -> usize {
        self.action_correct
    }

    pub fn action_correctness_rate(&self) -> f64 {
        rate(self.action_correct, self.total)
    }

    pub fn field_failures(&self) -> &BTreeMap<FieldPath, usize> {
        &self.field_failures
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
