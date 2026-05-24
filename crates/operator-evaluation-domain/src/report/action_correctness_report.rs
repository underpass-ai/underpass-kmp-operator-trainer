use std::collections::BTreeMap;

use operator_shared_domain::contract::correctness::field_path::FieldPath;
use operator_shared_domain::tool::kernel_tool::KernelTool;

use crate::report::field_stats::FieldStats;
use crate::report::tool_correctness_stats::ToolCorrectnessStats;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCorrectnessReport {
    total: usize,
    action_correct_count: usize,
    tool_selection_correct_count: usize,
    shape_invalid_count: usize,
    per_field_correctness: BTreeMap<FieldPath, FieldStats>,
    per_tool: BTreeMap<Option<KernelTool>, ToolCorrectnessStats>,
}

impl ActionCorrectnessReport {
    pub fn new(
        total: usize,
        action_correct_count: usize,
        tool_selection_correct_count: usize,
        shape_invalid_count: usize,
        per_field_correctness: BTreeMap<FieldPath, FieldStats>,
        per_tool: BTreeMap<Option<KernelTool>, ToolCorrectnessStats>,
    ) -> Self {
        Self {
            total,
            action_correct_count,
            tool_selection_correct_count,
            shape_invalid_count,
            per_field_correctness,
            per_tool,
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn action_correct_count(&self) -> usize {
        self.action_correct_count
    }

    pub fn tool_selection_correct_count(&self) -> usize {
        self.tool_selection_correct_count
    }

    pub fn shape_invalid_count(&self) -> usize {
        self.shape_invalid_count
    }

    pub fn action_correctness_rate(&self) -> f64 {
        rate(self.action_correct_count, self.total)
    }

    pub fn tool_selection_rate(&self) -> f64 {
        rate(self.tool_selection_correct_count, self.total)
    }

    pub fn shape_invalid_rate(&self) -> f64 {
        rate(self.shape_invalid_count, self.total)
    }

    pub fn per_field_correctness(&self) -> &BTreeMap<FieldPath, FieldStats> {
        &self.per_field_correctness
    }

    pub fn per_tool(&self) -> &BTreeMap<Option<KernelTool>, ToolCorrectnessStats> {
        &self.per_tool
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
