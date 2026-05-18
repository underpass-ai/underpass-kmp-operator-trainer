//! `StepKeyedPrediction` is the wire-level shape produced by an
//! external predictor (e.g. the Python `predict_operator_sft.py`
//! script) before it has been joined back to the ground-truth
//! trajectory. The key is the per-step identifier the predictor saw
//! in its input JSONL; the use case is responsible for resolving it
//! to a `TrainingTrajectoryId` via the ground-truth trajectory set.
//!
//! Once joined, this value becomes a `PredictedAction` and can be
//! paired with a `TrainingTrajectory` inside an `EvaluationPair`.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::step_id::StepId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepKeyedPrediction {
    step_id: StepId,
    action: OperatorAction,
}

impl StepKeyedPrediction {
    /// Public constructor on purpose. The two inputs are already
    /// validated value objects (`StepId` refuses empty / whitespace
    /// strings; `OperatorAction` is a closed enum whose every
    /// variant carries typed arguments validated at their own
    /// construction). There is no further invariant to enforce at
    /// this layer — sealing the constructor would just push noise to
    /// test code without strengthening the type.
    pub fn new(step_id: StepId, action: OperatorAction) -> Self {
        Self { step_id, action }
    }

    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }

    pub fn action(&self) -> &OperatorAction {
        &self.action
    }
}
